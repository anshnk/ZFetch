# ZFetch

**ZFetch** is a blazing-fast system information fetcher written in Rust. Inspired by tools like [neofetch](https://github.com/dylanaraps/neofetch) and [fastfetch](https://github.com/fastfetch-cli/fastfetch), it displays detailed system info alongside beautiful ASCII logos for various Linux distributions and operating systems.

## Features

* Rust-native performance ⚡
* Clean, minimal output
* ASCII logos per distro (thanks to FastFetch)
* Easy to extend and customize

## Example Output

```
                     ..'          ┌────────────────────────────────────────────────┐
                 ,xNMM.           │                System Information              │
               .OMMMMo            ├────────────────────────────────────────────────┤
               lMM"               │  Distro    : Mac OS (15.3.1)                   │
     .;loddo:.  .olloddol;.       │  Distro ID : macos                             │
   cKMMMMMMMMMMNWMMMMMMMMMM0:     │  Kernel    : 24.3.0                            │
 .KMMMMMMMMMMMMMMMMMMMMMMMWd.     │  CPU       : Apple M3 (8 cores) (4.06 GHz)     │
 XMMMMMMMMMMMMMMMMMMMMMMMX.       │  Memory    : 11.91 GB / 16.00 GB (74%)         │
;MMMMMMMMMMMMMMMMMMMMMMMM:        │  Swap      : 2.88 GB / 4.00 GB (72%)           │
:MMMMMMMMMMMMMMMMMMMMMMMM:        │  Local IP  : 192.0.0.2                         │
.MMMMMMMMMMMMMMMMMMMMMMMMX.       │  Battery   : 93% [Discharging]                 │
 kMMMMMMMMMMMMMMMMMMMMMMMMWd.     │  Uptime    : 24d 22h 17m                       │
 'XMMMMMMMMMMMMMMMMMMMMMMMMMMk    │  Disk (/)  : 427.52 GB / 460.43 GB (31%) - apfs│
  'XMMMMMMMMMMMMMMMMMMMMMMMMK.    └────────────────────────────────────────────────┘
    kMMMMMMMMMMMMMMMMMMMMMMd                                                        
     ;KMMMMMMMWXXWMMMMMMMk.                                                         
       "cooc*"    "*coo'"                                                             
```

## Configuration

ZFetch reads settings from a `config.json` file located in the same directory as the executable. The file is not created automatically and must be manually written by the user.

These are all the possible configurations you can make in it.

```json
{
  "show_distro": true,
  "show_distro_id": true,
  "show_kernel": true,
  "show_cpu": true,
  "show_gpu": true,
  "show_memory": true,
  "show_swap": true,
  "show_local_ip": true,
  "show_battery": true,
  "show_uptime": true,
  "logo_color": "#FF0000, #00FF00, #0000FF, #FFFF00, #00FFFF, #FF00FF",
  "color": "#FF5733",
  "show_user_host": true,
}
```

Each field toggles visibility or styling of specific system details:  

## Installation

Clone the repository and build with Cargo:

```bash
git clone https://github.com/anshnk/zfetch.git
cd zfetch
cargo build --release
./target/release/zfetch
```

## Roadmap

* [ ] Fix disk reading, its a little messed up lmao
* [x] More customization
* [ ] Make OS detection easier for ASCII
* [x] Speed up GPU Detection (i used iokit for this and other doohickery)

## Thanks

🎨 **Huge thanks to the [FastFetch](https://github.com/fastfetch-cli/fastfetch) project** for providing the ASCII logos used in this tool. Their work is greatly appreciated.

## License

MIT License

---

Made with 🦀 Rust