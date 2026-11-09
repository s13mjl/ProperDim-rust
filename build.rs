//! 把 `assets/app.ico` 编成 Win32 资源（RT_ICON + RT_GROUP_ICON）交给链接器，
//! 使 exe 在资源管理器/任务栏上带图标（对应 csproj 的 `<ApplicationIcon>`）。
//!
//! 本机没有 Windows SDK 的 `rc.exe`，所以直接手写 `.res`（RES 文件格式）：
//!   file header : DWORD DataSize=0, DWORD HeaderSize=32, 24 字节保留
//!   entry       : DWORD DataSize, DWORD HeaderSize, Type, Name,
//!                 DWORD DataVersion, WORD MemoryFlags, WORD LanguageId,
//!                 DWORD Version, DWORD Characteristics, <data>, 4 字节对齐
//! Type/Name 用 `0xFFFF + WORD 序号`; HeaderSize 不含前两个 DWORD。

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const LANG_EN_US: u16 = 0x0409;
const MEMFLAGS: u16 = 0x1030; // MOVEABLE | PURE | DISCARDABLE

fn u16le(v: u16) -> [u8; 2] { v.to_le_bytes() }
fn u32le(v: u32) -> [u8; 4] { v.to_le_bytes() }

fn push_ordinal(buf: &mut Vec<u8>, id: u16) {
    buf.extend_from_slice(&u16le(0xFFFF));
    buf.extend_from_slice(&u16le(id));
}

/// 追加一条资源；`type_id`/`name_id` 都是序号。
fn push_entry(out: &mut Vec<u8>, type_id: u16, name_id: u16, data: &[u8]) {
    let mut hdr = Vec::new();
    push_ordinal(&mut hdr, type_id);
    push_ordinal(&mut hdr, name_id);
    hdr.extend_from_slice(&u32le(0));            // DataVersion
    hdr.extend_from_slice(&u16le(MEMFLAGS));     // MemoryFlags
    hdr.extend_from_slice(&u16le(LANG_EN_US));   // LanguageId
    hdr.extend_from_slice(&u32le(0));            // Version
    hdr.extend_from_slice(&u32le(0));            // Characteristics

    out.extend_from_slice(&u32le(data.len() as u32));
    out.extend_from_slice(&u32le(hdr.len() as u32));
    out.extend_from_slice(&hdr);
    out.extend_from_slice(data);
    while out.len() % 4 != 0 { out.push(0); }
}

fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ico_path = Path::new(&manifest).join("assets").join("app.ico");
    let Ok(ico) = fs::read(&ico_path) else {
        println!("cargo:warning=assets/app.ico 不存在，跳过图标资源");
        return;
    };
    if ico.len() < 6 || u16::from_le_bytes([ico[2], ico[3]]) != 1 {
        println!("cargo:warning=assets/app.ico 不是有效 ICO，跳过图标资源");
        return;
    }
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;

    // 1. 解析 ICONDIR，按 (宽,高) 去重（保留后出现的，通常是 PNG 压缩的 256×256）
    struct Img { w: u8, h: u8, colors: u8, planes: u16, bpp: u16, bytes: Vec<u8> }
    let mut imgs: Vec<Img> = Vec::new();
    for i in 0..count {
        let o = 6 + i * 16;
        if o + 16 > ico.len() { break; }
        let w = ico[o];
        let h = ico[o + 1];
        let colors = ico[o + 2];
        let planes = u16::from_le_bytes([ico[o + 4], ico[o + 5]]);
        let bpp = u16::from_le_bytes([ico[o + 6], ico[o + 7]]);
        let size = u32::from_le_bytes([ico[o + 8], ico[o + 9], ico[o + 10], ico[o + 11]]) as usize;
        let off = u32::from_le_bytes([ico[o + 12], ico[o + 13], ico[o + 14], ico[o + 15]]) as usize;
        if off + size > ico.len() { continue; }
        imgs.push(Img { w, h, colors, planes, bpp, bytes: ico[off..off + size].to_vec() });
    }
    if imgs.is_empty() { return; }

    let mut seen: BTreeSet<(u8, u8)> = BTreeSet::new();
    let mut unique: Vec<Img> = Vec::new();
    for img in imgs.into_iter().rev() {
        if seen.insert((img.w, img.h)) { unique.push(img); }
    }
    unique.reverse();

    // 2. RT_ICON：序号从 1 开始
    let mut body = Vec::new();
    for (i, img) in unique.iter().enumerate() {
        push_entry(&mut body, RT_ICON, (i + 1) as u16, &img.bytes);
    }

    // 3. RT_GROUP_ICON：GRPICONDIR + 每项 GRPICONDIRENTRY（末位是图标序号而不是偏移）
    let mut grp = Vec::new();
    grp.extend_from_slice(&u16le(0));                 // reserved
    grp.extend_from_slice(&u16le(1));                 // type = icon
    grp.extend_from_slice(&u16le(unique.len() as u16));
    for (i, img) in unique.iter().enumerate() {
        grp.push(img.w);
        grp.push(img.h);
        grp.push(img.colors);
        grp.push(0);
        grp.extend_from_slice(&u16le(img.planes));
        grp.extend_from_slice(&u16le(img.bpp));
        grp.extend_from_slice(&u32le(img.bytes.len() as u32));
        grp.extend_from_slice(&u16le((i + 1) as u16));
    }
    push_entry(&mut body, RT_GROUP_ICON, 1, &grp);

    // 4. 组装 .res
    let mut res = Vec::new();
    res.extend_from_slice(&u32le(0));   // DataSize
    res.extend_from_slice(&u32le(32));  // HeaderSize
    res.extend_from_slice(&[0u8; 24]);
    res.extend_from_slice(&body);

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("app.res");
    if fs::write(&out, &res).is_ok() {
        println!("cargo:rustc-link-arg={}", out.display());
    }
}
