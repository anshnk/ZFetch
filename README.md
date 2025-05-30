# ZFetch

**ZFetch** is a blazing-fast system information fetcher written in Rust. Inspired by tools like [neofetch](https://github.com/dylanaraps/neofetch) and [fastfetch](https://github.com/fastfetch-cli/fastfetch), it displays detailed system info alongside beautiful ASCII logos for various Linux distributions and operating systems.

## Features

* Rust-native performance ⚡
* Clean, minimal output
* ASCII logos per distro (thanks to FastFetch)
* Easy to extend and customize

## Example Output

```
                     $1..'          ┌────────────────────────────────────────────────┐
                 ,xNMM.             │                System Information              │
               .OMMMMo              ├────────────────────────────────────────────────┤
               lMM"                 │  Distro    : Mac OS (15.3.1)                   │
     .;loddo:.  .olloddol;.         │  Distro ID : macos                             │
   cKMMMMMMMMMMNWMMMMMMMMMM0:       │  Kernel    : 24.3.0                            │
 $2.KMMMMMMMMMMMMMMMMMMMMMMMWd.     │  CPU       : Apple M3 (8 cores) (4.06 GHz)     │
 XMMMMMMMMMMMMMMMMMMMMMMMX.         │  GPU       : Apple M3                          │
$3;MMMMMMMMMMMMMMMMMMMMMMMM:        │  Memory    : 11.58 GB / 16.00 GB (72%)         │
:MMMMMMMMMMMMMMMMMMMMMMMM:          │  Swap      : 3.98 GB / 5.00 GB (80%)           │
.MMMMMMMMMMMMMMMMMMMMMMMMX.         │  Local IP  : 192.0.0.2                         │
 kMMMMMMMMMMMMMMMMMMMMMMMMWd.       │  Battery   : 100% [Discharging]                │
 $4'XMMMMMMMMMMMMMMMMMMMMMMMMMMk    │  Uptime    : 24d 21h 14m                       │
  'XMMMMMMMMMMMMMMMMMMMMMMMMK.      │  Disk (/)  : 427.60 GB / 460.43 GB (32%) - apfs│
    $5kMMMMMMMMMMMMMMMMMMMMMMd      └────────────────────────────────────────────────┘
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
  "logo_color": "#EABB97",
  "color": "#FF5733"
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
* [ ] More customization
* [ ] Make OS detection easier
* [ ] Speed up GPU Detection
      
## Thanks

🎨 **Huge thanks to the [FastFetch](https://github.com/fastfetch-cli/fastfetch) project** for providing the ASCII logos used in this tool. Their work is greatly appreciated.

## License

MIT License

---

Made with 🦀 Rust