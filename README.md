# Serial-rs
A cross-platform serial library for Rust

The main purpose of this library was to have a working serial library for Linux/Windows/OSX that works consistently between platforms, has overlapped IO on Windows, and also does not reboot devices when connecting (Arduino or ESP32).

This library follows the API and behaviour of pyserial

## Supported platforms
|Windows|Linux|OSX|BSD|Android|IOS|
|:-:|:-:|:-:|:-:|:-:|:-:|
|Yes|Yes|Yes|No|No|No|


