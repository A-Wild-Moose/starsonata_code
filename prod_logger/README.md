# Installing SS on headless Arm instance

`hangover: https://github.com/AndreRH/hangover`
wine for ARM

`Xvfb` virtual frame buffer
```bash
xvfb :0 -screen 0 1024x768x16
"ctrl+c"
DISPLAY=:0.0 wine starsonata2_installer.exe
```

Cygwin/Xserver for SSH with x11 forwarding to windows
