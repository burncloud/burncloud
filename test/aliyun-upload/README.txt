BurnCloud OpenClaw Offline Installation Test Package
====================================================

Contents:
- burncloud.exe: BurnCloud CLI (release build)
- openclaw-bundle: Offline bundle with Node.js 24.14.0

Bundle Details:
- Platform: Windows x64
- Node.js Version: 24.14.0
- Git Version: 2.53.0.2
- Total Size: ~123 MB

Installation Steps (on target Windows machine):
1. Copy burncloud.exe and openclaw-bundle folder to C:\burncloud-test\
2. Open Command Prompt as Administrator
3. Run: C:\burncloud-test\burncloud.exe install openclaw --bundle C:\burncloud-test\openclaw-bundle --auto-deps

NEW: Automatic PATH Setup
=========================
After installation, the installer will automatically add the following to your system PATH:
- Node.js bin directory
- Git cmd and bin directories
- npm global packages directory (for openclaw command)

IMPORTANT: You need to RESTART your terminal after installation for PATH changes to take effect.

Verification (after restarting terminal):
- node --version     -> v24.14.0
- git --version      -> git version 2.53.0.2
- openclaw --version -> OpenClaw version info

If PATH is not set correctly, you can manually run:
  setx PATH "%PATH%;<paths_shown_in_install_log>"
