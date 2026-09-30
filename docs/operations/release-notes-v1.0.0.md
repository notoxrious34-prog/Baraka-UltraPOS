# Baraka UltraPOS v1.0.0 — Notes de version / ملاحظات الإصدار

> **Audience : le gérant du magasin.** Ce document explique comment installer
> l'application, la démarrer pour la première fois, sauvegarder vos données et
> réagir en cas de problème. La version courte en arabe suit le texte français.

---

## 1. Installer l'application

1. Récupérez le fichier d'installation auprès de votre technicien :
   `Baraka UltraPOS_1.0.0_x64-setup.exe` (environ 140 Mo — il contient tout
   le nécessaire, **aucune connexion internet n'est requise** pour l'installer).
2. Double-cliquez sur le fichier et suivez l'assistant (français proposé
   en premier). L'installation est **par utilisateur** : installez-la depuis
   le compte Windows qui utilisera la caisse.
3. À la fin, « Baraka UltraPOS » apparaît dans le menu Démarrer.

### Avertissement Windows SmartScreen (normal)

L'installeur n'est pas signé numériquement : Windows affiche donc une fenêtre
bleue « Windows a protégé votre PC ». C'est attendu pour cette version.

1. Cliquez sur **« Informations complémentaires »**.
2. Cliquez sur **« Exécuter quand même »**.
3. L'installation démarre.

## 2. Premier démarrage — création du compte gérant

Au premier lancement, l'application crée sa base de données locale puis
affiche l'écran de configuration initiale :

1. Choisissez le nom d'utilisateur du compte **ADMIN** (le gérant).
2. Choisissez un mot de passe sûr et confirmez-le.
3. Validez : ce compte administre les utilisateurs, les paramètres et les
   sauvegardes. Conservez le mot de passe en lieu sûr.

Ce parcours est garanti par la suite de tests `first_run_bootstrap`
(`fresh_database_requires_setup_with_zero_users`,
`bootstrap_creates_exactly_one_active_admin_with_argon2id_hash`,
`second_bootstrap_is_rejected_with_zero_extra_rows`) : un seul compte
administrateur est créé, et toute tentative de double création est rejetée.

## 3. Vos données — où elles sont, comment les protéger

- Vos données (ventes, stock, clients) sont stockées **sur votre PC** dans :
  `%APPDATA%\com.baraka.ultrapos\baraka.db`
  (collez ce chemin dans l'explorateur Windows pour ouvrir le dossier).
- **Désinstaller l'application ne supprime jamais vos données** : le dossier
  ci-dessus est conservé.
- **Sauvegardez régulièrement** depuis l'application (fonction Sauvegarde) et
  copiez le fichier de sauvegarde sur une clé USB conservée hors du magasin.
  En cas de panne du PC, la restauration se fait depuis cette sauvegarde —
  demandez la procédure de restauration à votre technicien.

## 4. Imprimante ticket — mise en service

Avant d'imprimer les vrais tickets, faites réaliser avec votre technicien le
« drill » d'imprimante : `docs/operations/thermal-printer-drill.md`
(impression de test, vérification du ticket, signature). L'en-tête de vos
tickets (nom du magasin, NIF, NIS, RC, adresse) se configure dans
Paramètres → « Identité & Impression » (compte ADMIN requis).

## 5. Que faire si…

- **L'imprimante n'imprime plus** : vérifiez le câble et le papier, éteignez
  puis rallumez l'imprimante, puis relancez une impression de test depuis
  Paramètres → « Identité & Impression ». Si l'erreur persiste, vos ventes
  restent enregistrées : aucun ticket perdu n'est une vente perdue.
- **L'application ne démarre plus** : redémarrez le PC et réessayez. Si elle
  refuse toujours, **ne réinstallez pas avant d'avoir copié** le dossier
  `%APPDATA%\com.baraka.ultrapos` en lieu sûr, puis appelez le technicien.
- **Restaurer après une panne** : réinstallez l'application, restaurez la
  dernière sauvegarde via la fonction Restauration (compte ADMIN), vérifiez
  une vente et le stock avant de reprendre le service.

## 6. Ce que la v1 ne contient pas (en toute transparence)

- Pas de mise à jour automatique : les mises à jour se font en relançant un
  nouvel installeur par-dessus l'existant (vos données sont conservées).
- Pas de cloud ni de synchronisation : tout reste sur votre PC.
- Interface en français et en arabe uniquement.

---

## ملخص بالعربية

- **التثبيت**: شغّل `Baraka UltraPOS_1.0.0_x64-setup.exe` (يعمل دون إنترنت).
  عند ظهور تحذير SmartScreen الأزرق اضغط «Informations complémentaires» ثم
  «Exécuter quand même».
- **أول تشغيل**: أنشئ حساب المدير (ADMIN) واحفظ كلمة المرور في مكان آمن.
- **بياناتك** على جهازك في `%APPDATA%\com.baraka.ultrapos\baraka.db` — إلغاء
  التثبيت لا يحذفها أبدًا. انسخ نسخة احتياطية دوريًا على USB خارج المحل.
- **الطابعة**: اضبط ترويسة التذاكر (الإعدادات ← الهوية والطباعة، حساب مدير)
  ونفّذ تجربة الطباعة مع التقني قبل التشغيل الحقيقي.
- **عند مشكلة**: تحقق من الكابل والورق، أعد التشغيل، ولا تُعد التثبيت قبل نسخ
  مجلد البيانات، ثم اتصل بالتقني.
- **النسخة 1**: بلا تحديث تلقائي ولا سحابة، والواجهة بالفرنسية والعربية فقط.
