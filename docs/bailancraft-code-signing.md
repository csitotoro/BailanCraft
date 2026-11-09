# การเซ็นโค้ด BailanCraft (Code signing)

ถ้าไม่เซ็นโค้ด เวลาผู้ใช้ดับเบิลคลิกตัวติดตั้งหรือ `BailanCraft.exe` ครั้งแรก Windows SmartScreen จะขึ้นว่า
**"Windows protected your PC"** และผู้ใช้ต้องกด *More info → Run anyway* เอง
บางบริษัทตั้งนโยบายให้รันได้เฉพาะโปรแกรมที่เซ็นแล้ว การเซ็นโค้ดจะแก้ทั้งสองเรื่องนี้

## ทางเลือก

| ทางเลือก | ค่าใช้จ่ายโดยประมาณ / ปี | เหมาะกับ | หมายเหตุ |
|---|---|---|---|
| **ใช้ภายในบริษัทเท่านั้น**: ใบรับรองจาก CA ภายในของบริษัท (Active Directory Certificate Services) | ไม่มี (ถ้ามี AD CS อยู่แล้ว) | แจกเฉพาะเครื่องในโดเมนบริษัท | ฝ่าย IT ต้อง deploy ใบรับรองเป็น Trusted Publisher ผ่าน Group Policy ใช้กับเครื่องนอกบริษัทไม่ได้ |
| **Azure Trusted Signing** (Microsoft) | ประมาณ 10 USD/เดือน | แจกทั่วไป บริษัทจดทะเบียนแล้ว | ต้องยืนยันตัวตนบริษัท ไม่ต้องใช้ฮาร์ดแวร์ เป็นทางที่ง่ายและถูกที่สุดตอนนี้ |
| **OV / EV Code Signing Certificate** (DigiCert, Sectigo, GlobalSign ฯลฯ) | ประมาณ 200–700 USD | แจกทั่วไป | ตั้งแต่ปี 2023 ต้องเก็บกุญแจใน USB token หรือ cloud HSM ส่วน EV ทำให้ SmartScreen เชื่อถือเร็วกว่า |

สำหรับบริษัทที่เริ่มแจกภายในก่อน ผมแนะนำ **CA ภายในบริษัท** ถ้าฝ่าย IT มี AD CS อยู่แล้ว
ถ้าจะแจกให้คนนอกบริษัทด้วย แนะนำ **Azure Trusted Signing**

## วิธีเซ็นตอนสร้างตัวติดตั้ง

`packaging/windows/bailancraft-package.ps1` เรียก `bailancraft-sign.ps1` ให้เองทั้งกับไฟล์ .exe และ .msi
เมื่อตั้งค่าตัวแปรใดตัวแปรหนึ่งต่อไปนี้ก่อนรัน:

```powershell
# ใบรับรองที่อยู่ใน Windows certificate store (USB token, CA ภายใน, หรือ cloud HSM ที่ติดตั้ง CSP ไว้)
$env:BAILANCRAFT_SIGN_THUMBPRINT = '<SHA-1 thumbprint ของใบรับรอง>'

# หรือไฟล์ .pfx (เฉพาะ CA ภายในบริษัท ใบรับรองสาธารณะสมัยนี้ไม่ออกเป็นไฟล์แล้ว)
$env:BAILANCRAFT_SIGN_PFX = 'C:\path\to\cert.pfx'
$env:BAILANCRAFT_SIGN_PFX_PASSWORD = '<รหัสผ่าน>'

# (ไม่บังคับ) เซิร์ฟเวอร์ timestamp ค่าเริ่มต้นคือ http://timestamp.digicert.com
$env:BAILANCRAFT_SIGN_TIMESTAMP = 'http://timestamp.digicert.com'
```

ถ้าไม่ได้ตั้งค่าไว้ สคริปต์จะเตือนแล้วสร้างตัวติดตั้งแบบไม่เซ็นไปตามปกติ

Azure Trusted Signing ใช้ `signtool` กับ dlib ของ Microsoft (`/dlib Azure.CodeSigning.Dlib.dll /dmdf metadata.json`)
แทนการชี้ใบรับรอง ถ้าเลือกทางนี้ ต้องเพิ่มคำสั่งดังกล่าวใน `bailancraft-sign.ps1`

## ตรวจว่าเซ็นแล้ว

```powershell
Get-AuthenticodeSignature .\dist\bailancraft\bailancraft-*-windows-x64.msi | Format-List Status, SignerCertificate
```

`Status` ต้องเป็น `Valid`

## ข้อควรระวัง

- อย่าเก็บไฟล์ .pfx หรือรหัสผ่านไว้ใน repository
- ใส่ timestamp ทุกครั้ง (สคริปต์ใส่ให้แล้ว) เพื่อให้ลายเซ็นยังใช้ได้หลังใบรับรองหมดอายุ
- ชื่อผู้เผยแพร่ใน MSI มาจาก `-Manufacturer` ของ `bailancraft-package.ps1` ควรตรงกับชื่อบริษัทในใบรับรอง
