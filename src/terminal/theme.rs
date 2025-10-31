use std::env;
use std::ffi::CString;
use std::io;
use std::mem;

#[cfg(unix)]
use libc::{c_int, termios};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct TerminalColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub dark: bool,
}

impl TerminalColor {
    fn new(r: u8, g: u8, b: u8) -> Self {
        let dark = is_dark(r, g, b);
        Self { r, g, b, dark }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct TerminalTheme {
    pub fg: TerminalColor,
    pub bg: TerminalColor,
}

pub fn detect_terminal_theme(force_env: bool) -> Option<TerminalTheme> {
    if !force_env {
        if let Ok(theme) = detect_by_escape_code() {
            return Some(theme);
        }
    }
    detect_by_env()
}

#[cfg(not(unix))]
fn detect_by_escape_code() -> io::Result<TerminalTheme> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "terminal detection only supported on Unix",
    ))
}

#[cfg(unix)]
fn detect_by_escape_code() -> io::Result<TerminalTheme> {
    use libc::{self, c_void, O_CLOEXEC, O_NOCTTY, O_RDWR, TCSAFLUSH};

    const QUERY: &[u8] = b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\";
    const TIMEOUT_MS: i32 = 100;

    let path = CString::new("/dev/tty").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), O_RDWR | O_NOCTTY | O_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    struct FdGuard(c_int);
    impl Drop for FdGuard {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.0);
            }
        }
    }

    let _fd_guard = FdGuard(fd);

    let mut original: termios = unsafe { mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut original) } == -1 {
        return Err(io::Error::last_os_error());
    }

    let mut raw = original;
    raw.c_lflag &= !(libc::ICANON | libc::ECHO);
    raw.c_cc[libc::VMIN] = 0;
    raw.c_cc[libc::VTIME] = 0;

    if unsafe { libc::tcsetattr(fd, TCSAFLUSH, &raw) } == -1 {
        return Err(io::Error::last_os_error());
    }

    struct TermGuard {
        fd: c_int,
        original: termios,
    }

    impl Drop for TermGuard {
        fn drop(&mut self) {
            unsafe {
                libc::tcsetattr(self.fd, TCSAFLUSH, &self.original);
            }
        }
    }

    let _guard = TermGuard { fd, original };

    let mut written = 0;
    while written < QUERY.len() {
        let slice = &QUERY[written..];
        let rc = unsafe { libc::write(fd, slice.as_ptr() as *const c_void, slice.len()) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        written += rc as usize;
    }

    if !wait_for_ready(fd, TIMEOUT_MS)? {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "terminal did not respond to OSC query",
        ));
    }

    let mut buffer = [0u8; 1024];
    let mut bytes = 0usize;

    loop {
        let rc = unsafe {
            libc::read(
                fd,
                buffer[bytes..].as_mut_ptr() as *mut c_void,
                buffer.len() - bytes - 1,
            )
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 {
            break;
        }
        bytes += rc as usize;
        buffer[bytes] = 0;

        if let Some(theme) = parse_osc_response(&buffer[..bytes]) {
            return Ok(theme);
        }

        if bytes >= buffer.len() - 1 {
            break;
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "failed to parse terminal response",
    ))
}

#[cfg(unix)]
fn wait_for_ready(fd: c_int, timeout_ms: i32) -> io::Result<bool> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        use libc::{select, timeval, FD_SET, FD_ZERO};

        let mut set = unsafe { mem::zeroed() };
        unsafe {
            FD_ZERO(&mut set);
            FD_SET(fd, &mut set);
        }

        let mut tv = timeval {
            tv_sec: (timeout_ms / 1000) as libc::time_t,
            tv_usec: (timeout_ms % 1000) * 1000,
        };

        let rc = unsafe {
            select(
                fd + 1,
                &mut set,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            )
        };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(rc > 0)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        use libc::{poll, pollfd, POLLIN};

        let mut fds = pollfd {
            fd,
            events: POLLIN,
            revents: 0,
        };
        let rc = unsafe { poll(&mut fds, 1, timeout_ms) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(rc > 0)
    }
}

fn parse_osc_response(buffer: &[u8]) -> Option<TerminalTheme> {
    let response = std::str::from_utf8(buffer).ok()?;
    let fg = extract_color(response, "\x1b]10;rgb:")?;
    let bg = extract_color(response, "\x1b]11;rgb:")?;

    let fg = TerminalColor::new(fg.0, fg.1, fg.2);
    let bg = TerminalColor::new(bg.0, bg.1, bg.2);

    Some(TerminalTheme { fg, bg })
}

fn extract_color<'a>(response: &'a str, prefix: &str) -> Option<(u8, u8, u8)> {
    let start = response.find(prefix)?;
    let after_prefix = &response[start + prefix.len()..];
    let terminator = "\x1b\\";
    let end = after_prefix.find(terminator)?;
    parse_triplet(&after_prefix[..end])
}

fn parse_triplet(spec: &str) -> Option<(u8, u8, u8)> {
    let mut parts = spec.split('/');
    let r = parse_component(parts.next()?)?;
    let g = parse_component(parts.next()?)?;
    let b = parse_component(parts.next()?)?;
    Some((r, g, b))
}

fn parse_component(component: &str) -> Option<u8> {
    if component.is_empty() {
        return None;
    }
    let mut value = u32::from_str_radix(component, 16).ok()?;
    if value > 0x0100 {
        value /= 0x0100;
    }
    if value > 0xFF {
        value = 0xFF;
    }
    Some(value as u8)
}

fn detect_by_env() -> Option<TerminalTheme> {
    let value = env::var("COLORFGBG").ok()?;
    let mut parts = value.split(';');
    let fg = parts.next()?.parse::<i32>().ok()?;
    let bg = parts.next()?.parse::<i32>().ok()?;
    Some(TerminalTheme {
        fg: ansi_color_to_rgb(fg)?,
        bg: ansi_color_to_rgb(bg)?,
    })
}

fn ansi_color_to_rgb(index: i32) -> Option<TerminalColor> {
    if index < 0 {
        return None;
    }
    let (r, g, b) = match index {
        0 => (0, 0, 0),
        1 => (205, 0, 0),
        2 => (0, 205, 0),
        3 => (205, 205, 0),
        4 => (0, 0, 238),
        5 => (205, 0, 205),
        6 => (0, 205, 205),
        7 => (229, 229, 229),
        8 => (127, 127, 127),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (92, 92, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        _ => return None,
    };
    Some(TerminalColor::new(r, g, b))
}

fn is_dark(r: u8, g: u8, b: u8) -> bool {
    (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) < 128_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_threshold() {
        assert!(is_dark(0, 0, 0));
        assert!(!is_dark(255, 255, 255));
    }
}
