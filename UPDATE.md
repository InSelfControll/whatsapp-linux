# WhatsApp Desktop - Update Notes

## Current Status

### External Links - COMPLETED
- Clicking on external links (Zoom, YouTube, etc.) now opens them in your default browser
- Works with `xdg-open` on Linux, `open` on macOS, `start` on Windows

### PDF Files - COMPLETED
- PDF files now open in your preferred browser (Brave / Firefox / Chrome / Chromium)
- On first PDF download, a dialog prompts you to select your preferred browser
- If only one browser is installed, it's used automatically
- If no browsers are detected, falls back to system default (`xdg-open`)
- **Configuration is saved** to `~/.config/whatsapp-desktop/config.json`

### Document Files (Office/Docs) - COMPLETED
- Document files (Excel, Word, PowerPoint, etc.) now offer two options:
  - **Google Docs**: Opens the appropriate Google Docs/Sheets/Slides create page + shows notification to upload the file
  - **Local Application**: Opens with system default application
- Supported formats: `.doc`, `.docx`, `.xls`, `.xlsx`, `.ppt`, `.pptx`, `.odt`, `.ods`, `.odp`
- On first document download, a dialog prompts you to choose your preference
- **Configuration is saved** and remembered for future downloads

### Session Persistence - COMPLETED
- Session data (login, last chat, preferences) now persists between app restarts
- WebContext uses persistent data directory at `~/.local/share/whatsapp-desktop/webdata/`
- Cookies stored at `~/.local/share/whatsapp-desktop/cookies.txt`
- Local storage and IndexedDB are enabled and persistent

### Clipboard Paste (Images) - IN PROGRESS
- JavaScript paste handler added to intercept Ctrl+V events
- Attempts to inject pasted images into WhatsApp's file input
- **Status**: Needs testing - may require additional debugging

### Notification Mute - COMPLETED
- Press **Ctrl+Shift+M** to toggle notification mute on/off
- When muted:
  - Desktop notifications are blocked
  - Notification sounds are silenced
  - Voice messages and videos in chats **still play with audio**
- Mute preference is saved and persists between sessions
- Visual indicator shows mute status when toggling

### Video Calls and Voice Calls - WORKAROUND IMPLEMENTED
- **Issue**: When attempting a voice/video call, WhatsApp shows "Get the macOS app"
- **Workaround implemented**: App now spoofs as official WhatsApp macOS Desktop (Apple Silicon M1/M2)
  - User-Agent mimics WhatsApp Desktop Electron on macOS
  - Navigator properties report macOS platform
  - Electron-specific globals added (`window.process`, `window.electronAPI`)
  - ARM64 architecture reported for Apple Silicon compatibility
- **Note**: This workaround may or may not work depending on WhatsApp's fingerprinting. Testing required.

## File Handling Summary

| File Type | Behavior | Configuration |
|-----------|----------|---------------|
| PDF | Browser selection (Brave/Firefox/Chrome/Chromium) | Saved to config |
| Word/Excel/PPT | Google Docs or Local App | Saved to config |
| Images | Stays in app | - |
| Audio/Video | Stays in app | - |
| ZIP | Opens with system default | - |

## Configuration

User preferences are stored in:
```
~/.config/whatsapp-desktop/config.json
```

Example config:
```json
{
  "pdf_browser": "Firefox",
  "doc_handler": "GoogleDocs"
}
```

To reset preferences, delete this file.

## Dialog System

The app uses `zenity` (GNOME) or `kdialog` (KDE) for user prompts:
- Automatically detects which is available
- Falls back to sensible defaults if neither is installed

## Technical Details

### Browser Detection
Checks for installed browsers using `which`:
- `brave-browser`
- `firefox`
- `google-chrome`
- `chromium`

### macOS Spoofing (for calls)
```
User-Agent: WhatsApp/2.24.6.80 Chrome/120.0.6099.291 Electron/28.2.5
Platform: MacIntel (Apple Silicon compatibility)
Architecture: arm64
Electron version: 28.2.5
```

## Known Limitations

1. **Voice/Video calls**: May still not work if WhatsApp uses additional fingerprinting beyond what we spoof
2. **Google Docs**: Cannot directly open local files; user must upload manually after Google Docs opens
3. **Linux only**: Browser detection and dialog features are Linux-specific; other platforms use system defaults
