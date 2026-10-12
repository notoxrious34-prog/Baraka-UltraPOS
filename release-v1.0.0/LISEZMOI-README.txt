Baraka UltraPOS v1.0.0 - Quick Start / دليل البدء السريع
=====================================================

FILE / الملف
  Baraka UltraPOS_1.0.0_x64-setup.exe (~211 MB, 221552107 bytes)
  SHA-256: C608C32A8C96B089CB51E187DE473AD0FCCD3E728E8B1C07D0311C1E65D75934
  Verify: certutil -hashfile "Baraka UltraPOS_1.0.0_x64-setup.exe" SHA256

INSTALL (Windows 10/11, no internet needed)
  1. Double-click the setup file (French offered first).
  2. Install from the Windows account that will run the register.
  3. Blue SmartScreen warning is expected (unsigned installer):
     "Informations complémentaires" -> "Exécuter quand même".

FIRST LAUNCH / أول تشغيل
  1. Create the ADMIN (manager) account; keep the password safe.
  2. Settings -> "Identité & Impression": store name, address, phone,
     fiscal identifiers RC / NIF / NIS / AI (printed on tickets).
  3. Language switch FR/AR is at the top (Arabic renders right-to-left).

PRINTER / الطابعة
  1. Recommended: 80mm ESC/POS thermal printer on a serial or virtual-COM
     USB port. Note the COM port (e.g. COM3) in Windows Device Manager.
  2. Enter port + baud rate in Settings -> "Identité & Impression".
  3. Use the test-print button (accounting-free diagnostic), then run the
     physical drill with your technician before live service.

DAILY USE / الاستعمال اليومي
  Open cash session -> sell (scan/search) -> F8 parks a sale, F6 resumes ->
  collect debts via client dossier (سند قبض, overpayment blocked) ->
  close session and count the drawer (daily Z-report).

DATA SAFETY / سلامة البيانات
  Database lives at %APPDATA%\com.baraka.ultrapos\baraka.db and is NEVER
  deleted by install, reinstall, or uninstall. Back up from Settings ->
  Backups and copy the file to a USB stick kept outside the shop.

DOCS
  Full handbook: GUIDE-UTILISATION.html (offline, in this folder).
  Printer drill: docs/operations/thermal-printer-drill.md
  Release notes: docs/operations/release-notes-v1.0.0.md
