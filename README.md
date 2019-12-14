# Upload stick

Utility to monitor files added to a block device and upload any WAV audio files
found.

<https://github.com/JoelColledge/meta-pi-upload-stick> defines an image using
this utility which turns a Raspberry Pi Zero W into an auto-uploading USB
stick.

## Components

### `upload_stick_prepare`

Should be run once to set up backing devices for this to act as a mass storage
device.

### `upload_stick_start`

Should be run on each boot to start mass storage.

### `upload_stick_run`

Monitors activity on the mass storage device and uploads new files found.
