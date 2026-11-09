# BailanCraft (ใบลาน)

โปรแกรมอ่านและแก้ไข PDF ที่รองรับภาษาไทยเต็มรูปแบบ ทั้งพยัญชนะ สระ และวรรณยุกต์
พัฒนาต่อยอดจาก [PdfCraft](https://github.com/storytold/pdfcraft) (MIT OR Apache-2.0) ด้วยภาษา Rust

A PDF reader and editor with first-class Thai support, forked from
[PdfCraft](https://github.com/storytold/pdfcraft) and released under the same licences.

**พัฒนาโดย LANNAFORGE** — *Born in Chiang Mai, Inspired by Lanna, Built for the World.*

**ดาวน์โหลด:** [Releases](https://github.com/csitotoro/BailanCraft/releases) ·
**แจ้งปัญหา / ส่งความเห็น:** [Issues](https://github.com/csitotoro/BailanCraft/issues)
ในโปรแกรม: เมนู วิธีใช้ → Report a problem or send feedback

## สถานะ

| เรื่อง | สถานะ |
|---|---|
| เมนูภาษาไทย, เพิ่มข้อความไทย, copy/ค้นหาข้อความไทย | ใช้ได้ (มาจาก PdfCraft) |
| ชื่อ/โฟลเดอร์ค่าตั้งแยกจาก PdfCraft, ปิดตัวเช็กอัปเดตของต้นฉบับ | ทำแล้ว |
| ไอคอนมัดใบลาน, ธีมใบลานธรรมชาติ / ประกับไม้ | ทำแล้ว |
| ตัดคำไทยตามพจนานุกรม (PDF และหน้าจอ) | ทำแล้ว |
| ภาษาไทยในฟอร์ม, Fill & Sign, หัว/ท้ายกระดาษ, ลายน้ำ, ตรายาง | ทำแล้ว (มีชุดตรายางภาษาไทย 12 แบบ) |
| OCR ภาษาไทย | ทำแล้ว (Tesseract + tha.traineddata มากับตัวติดตั้ง) |
| ตัวติดตั้ง Windows (.msi) และแบบพกพา (.zip) | ทำแล้ว: `packaging/windows/bailancraft-package.ps1` |
| ลายเซ็นแบบพิมพ์ชื่อภาษาไทย | ยังไม่ทำ (ต้องหาฟอนต์ลายมือไทย) |

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

## สร้างตัวติดตั้ง Windows

ต้องมี WiX v5 (`dotnet tool install --global wix --version 5.0.2`), Tesseract
(`winget install UB-Mannheim.TesseractOCR`) และ `tha.traineddata`, `eng.traineddata` จาก
[tessdata_best](https://github.com/tesseract-ocr/tessdata_best) ใน `..\BailanCraft-deps\tessdata`

```powershell
$env:CRAFT_FONTS_DIR = '..\craft-fonts'; $env:CRAFT_I18N_DIR = '..\craft-i18n'
powershell -ExecutionPolicy Bypass -File packaging\windows\bailancraft-package.ps1 -Manufacturer 'ชื่อบริษัท'
```

ได้ `dist\bailancraft\bailancraft-<version>-windows-x64.msi` และไฟล์ `-portable.zip`
การเซ็นโค้ดดูที่ [docs/bailancraft-code-signing.md](docs/bailancraft-code-signing.md)
