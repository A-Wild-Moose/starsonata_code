# Installing SS on headless Arm instance

`hangover: https://github.com/AndreRH/hangover`
wine for ARM

`Xvfb` virtual frame buffer
```bash
xvfb-run -f <auth-file> -a <program/command>
```

```bash
ps -ef|grep Xvfb  # to find the screen/display variable
DISPLAY=:X.Y XAUTHORITY=<auth-file> xdotool search --name Sonata  # find the window ID for star sonata
```


This not preferred as it leaves Xvfb running
```bash
Xvfb :0 -screen 0 1024x768x16
"ctrl+c"
DISPLAY=:0.0 wine starsonata2_installer.exe
```

Cygwin/Xserver for SSH with x11 forwarding to windows


Solution for handling no permissions on socket capturing
```bash
sudo setcap cap_net_raw,cap_net_admin=eip /path/to/your/binary
```
