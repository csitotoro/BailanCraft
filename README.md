# BailanCraft (ใบลาน)

โปรแกรมอ่านและแก้ไข PDF ที่รองรับภาษาไทยเต็มรูปแบบ ทั้งพยัญชนะ สระ และวรรณยุกต์
พัฒนาต่อยอดจาก [PdfCraft](https://github.com/storytold/pdfcraft) (MIT OR Apache-2.0) ด้วยภาษา Rust

A PDF reader and editor with first-class Thai support, forked from
[PdfCraft](https://github.com/storytold/pdfcraft) and released under the same licences.

## สถานะ

| เรื่อง | สถานะ |
|---|---|
| เมนูภาษาไทย, เพิ่มข้อความไทย, copy/ค้นหาข้อความไทย | ใช้ได้ (มาจาก PdfCraft) |
| ชื่อ/โฟลเดอร์ค่าตั้งแยกจาก PdfCraft, ปิดตัวเช็กอัปเดตของต้นฉบับ | ทำแล้ว |
| ไอคอนและธีมใบลาน | กำลังออกแบบ (ตอนนี้ยังใช้ไอคอนของต้นฉบับ) |
| ตัดคำไทยตามพจนานุกรม | ยังไม่ทำ (ตอนนี้ตัดกลางคำได้) |
| OCR ไทย, ภาษาไทยในฟอร์มและ Fill & Sign | ยังไม่ทำ |
| ตัวติดตั้ง Windows | ยังไม่ทำ |

## Build (Windows)

ต้องมี Rust 1.95+ และ Visual Studio 2022 Build Tools (C++) แล้ววาง build input สองตัวไว้ข้างโฟลเดอร์นี้:

```sh
git clone https://github.com/storytold/craft-fonts ../craft-fonts      # commit ตาม .github/workflows/ci.yml (craft-fonts-ref)
git clone https://github.com/storytold/craft-i18n ../craft-i18n        # commit ตาม craft-i18n.rev
```

```sh
CRAFT_FONTS_DIR=../craft-fonts CRAFT_I18N_DIR=../craft-i18n cargo build --release -p pdfcraft -j 8
```

ได้ไฟล์ `target/release/pdfcraft.exe` (ชื่อไฟล์ .exe จะเปลี่ยนตอนทำตัวติดตั้ง)
ถ้าไม่ตั้ง `CRAFT_FONTS_DIR` โปรแกรมจะไม่มีฟอนต์ไทย และเพิ่มข้อความไทยลงใน PDF ไม่ได้

## ลิขสิทธิ์

MIT OR Apache-2.0 เหมือนต้นฉบับ (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md`)
ชื่อและโลโก้ ArtCraft เป็นเครื่องหมายการค้าของ ArtCraft Team และไม่ได้รวมอยู่ใน BailanCraft
README เดิมของ PdfCraft อยู่ที่ [docs/UPSTREAM-README.md](docs/UPSTREAM-README.md)
