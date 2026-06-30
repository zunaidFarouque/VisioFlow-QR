# Publishing the GitHub Wiki

The wiki pages in this folder are ready to publish. GitHub does not create the `.wiki.git` repository until **at least one page** is created through the web UI.

## Step 1 — Initialize the wiki (one-time, ~30 seconds)

1. Open https://github.com/zunaidFarouque/VisioFlow-QR/wiki
2. Click **Create the first page**
3. Set title to `Home`, body to `init`, click **Save page**

This creates the wiki git backend. You can overwrite `Home` in the next step.

## Step 2 — Push all pages

From PowerShell:

```powershell
cd "d:\_installed\VScode repos\AI\Cursor\VisioFlow-QR-wiki"

# If the wiki repo was not cloned yet:
git clone https://github.com/zunaidFarouque/VisioFlow-QR.wiki.git .

# Or copy from this folder:
Copy-Item -Path "..\wiki\*.md" -Destination . -Force

git add -A
git commit -m "Official VisioFlow-QR wiki documentation"
git push -u origin master
```

If `master` is rejected, try:

```powershell
git push -u origin master:main
```

### Authenticated push (if clone/push fails)

```powershell
$token = gh auth token
git remote set-url origin "https://x-access-token:${token}@github.com/zunaidFarouque/VisioFlow-QR.wiki.git"
git push -u origin master
```

## Step 3 — Verify

```powershell
gh api repos/zunaidFarouque/VisioFlow-QR/wiki/pages
```

Or open https://github.com/zunaidFarouque/VisioFlow-QR/wiki/Home

## Pages included

| File | Wiki page |
|------|-----------|
| `Home.md` | Landing page |
| `Installation.md` | Install paths |
| `Quick-Start.md` | First scan |
| `Capture.md` | Capture flags |
| `Notifications.md` | Windows toasts |
| `Routing-and-Auto-Route.md` | Auto-routing |
| `Default-Rules.md` | Stock rules |
| `Custom-Rules.md` | Rule CRUD |
| `CLI-Reference.md` | Full CLI |
| `Daemon-and-IPC.md` | Daemon + IPC |
| `Distribution-and-Release.md` | Releases |
| `_Sidebar.md` | Sidebar navigation |

Local commit (pre-push): `8bf2a27` in `VisioFlow-QR-wiki/`
