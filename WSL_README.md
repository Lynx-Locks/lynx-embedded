# WSL Setup for Embedded Development

By default, WSL does not have access to any USB devices.
You will need to use the following workaround if you want to run the program through WSL.

Take a look at [this Microsoft Article](https://learn.microsoft.com/en-us/windows/wsl/connect-usb) if you need additional help.

The following instructions are for Ubuntu 20.04.

## First Setup

1. [Install the latest version of usbidp](https://github.com/dorssel/usbipd-win/releases)

2. In WSL:
    ```bash
    sudo apt install linux-tools-virtual hwdata
    sudo update-alternatives --install /usr/local/bin/usbip usbip `ls /usr/lib/linux-tools/*/usbip | tail -n1` 20
    ```

3. Run: `usbipd.exe wsl list`. This will list all your USB devices.
   Find the ESP32's USB to UART Bridge. Note its `BUSID` (something like `2-2`).

   Optionally, you can export it as an environment variable:
   ```bash
   # Example. Replace "2-2" with your BUSID.
   export BUSID="2-2"
   ```

4. Attach the device to WSL:
    ```bash
    usbipd.exe wsl attach --busid=$BUSID
    ```

5. Run `usbipd.exe wsl list` again and confirm that the USB to UART Bridge state is now attached to Ubuntu.

6. Done! You can now run the program from WSL.

> **Note**
> If you are having issues, try running `sudo service udev restart`.
> Then run `usbipd.exe wsl detach --busid=$BUSID && usbipd.exe wsl attach --busid=$BUSID`
> (effectively unplugging then plugging the USB back in).

## Subsequent Setups

If you ever unplug the ESP32 from your computer, you will need to do a couple of commands before you can run the application.

1. Get the `BUSID`:
    ```bash
    usbipd.exe wsl list
    ```

2. Attach the usb to WSL:
    ```bash
    usbipd.exe wsl attach --busid=$BUSID
    ```
