//! Swagger UI & OpenAPI 3.0.3 Specification Provider
//!
//! Exposes interactive Swagger UI documentation and raw OpenAPI JSON specification
//! for all Raspberry Pi Zero Extended Monitor receiver API endpoints.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

pub const OPENAPI_JSON: &str = r##"{

  "openapi": "3.0.3",
  "info": {
    "title": "Pi Zero Extended Monitor - Receiver Control API",
    "description": "Comprehensive REST and telemetry API for the Raspberry Pi Zero GPU Display Receiver appliance.\nSupports real-time telemetry, stream parameter hot-tuning, hardware decoding controls, diagnostics, storage management, and firmware updates.",
    "version": "2.3.0",
    "contact": {
      "name": "Carlos Alberto",
      "email": "psncarlosalberto4ti@gmail.com"
    },
    "license": {
      "name": "MIT"
    }
  },
  "servers": [
    {
      "url": "http://192.168.7.2:8080",
      "description": "Pi Zero Direct USB Virtual Gateway (Default)"
    },
    {
      "url": "http://localhost:8080",
      "description": "Local Port-Forward / Direct Access"
    }
  ],
  "tags": [
    {
      "name": "Status & Telemetry",
      "description": "Real-time SoC temperature, CPU utilization, free RAM, and display status"
    },
    {
      "name": "IoT Media & HDMI Visualizer",
      "description": "Real-time 24-band audio spectrum, UPnP DLNA / DIAL metadata, and dynamic HDMI visualizer"
    },
    {
      "name": "Host Remote Control",
      "description": "Bidirectional transmitter control on Host PC (Start/Stop, Extend/Clone, Audio, and HUD)"
    },
    {
      "name": "Audio & Sound",
      "description": "Digital HDMI audio volume, mute, and BlueZ A2DP Bluetooth discovery"
    },
    {
      "name": "Stream Configuration",
      "description": "Adaptive bitrate, framerate, HUD overlays, and color profiles"
    },
    {
      "name": "Operating Modes",
      "description": "Switch between USB Bulk Direct, Miracast RTSP, and UDP Network"
    },
    {
      "name": "Stream Playback",
      "description": "Pause, resume, and manage hardware V4L2 M2M decode pipeline"
    },
    {
      "name": "System & Firmware",
      "description": "Reboot, graceful shutdown, and Over-The-Air (OTA) appliance updates"
    },
    {
      "name": "Diagnostics & Logs",
      "description": "Kernel dmesg, receiver logs, remote command execution, and framebuffer capture"
    },
    {
      "name": "Micro-SD Storage",
      "description": "Mount, unmount, and inspect the physical boot partition from RAM"
    },
    {
      "name": "Networking",
      "description": "USB Ethernet Zero-Gateway and WiFi configuration"
    },
    {
      "name": "Client Downloads",
      "description": "Host drivers, portable binaries, and auto-connect scripts"
    },
    {
      "name": "Web Dashboard & Documentation",
      "description": "Interactive HTML control panel and OpenAPI schema endpoints"
    }
  ],
  "paths": {
    "/": {
      "get": {
        "tags": ["Web Dashboard & Documentation"],
        "summary": "Embedded Web Control Dashboard",
        "description": "Returns the complete single-page application (HTML/CSS/JS) embedded in binary memory for device telemetry, display preview, and hot-tuning.",
        "operationId": "getDashboard",
        "responses": {
          "200": {
            "description": "Dashboard HTML document",
            "content": {
              "text/html; charset=utf-8": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      }
    },
    "/swagger": {
      "get": {
        "tags": ["Web Dashboard & Documentation"],
        "summary": "Swagger UI Interactive Documentation",
        "description": "Serves this dark-themed interactive API explorer and documentation interface.",
        "operationId": "getSwaggerUI",
        "responses": {
          "200": {
            "description": "Swagger UI HTML document",
            "content": {
              "text/html; charset=utf-8": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      }
    },
    "/api/openapi.json": {
      "get": {
        "tags": ["Web Dashboard & Documentation"],
        "summary": "OpenAPI 3.0.3 Specification Document",
        "description": "Returns the complete OpenAPI 3.0.3 machine-readable JSON schema specification.",
        "operationId": "getOpenApiSpec",
        "responses": {
          "200": {
            "description": "OpenAPI 3.0.3 JSON schema",
            "content": {
              "application/json; charset=utf-8": {
                "schema": {
                  "type": "object"
                }
              }
            }
          }
        }
      }
    },
    "/api/status": {
      "get": {
        "tags": ["Status & Telemetry"],
        "summary": "Real-time Hardware & Stream Status",
        "description": "Returns current SoC temperature in Celsius, CPU usage percentage, free RAM in MiB, and stream playback state.",
        "operationId": "getStatus",
        "responses": {
          "200": {
            "description": "Live telemetry metrics",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/StatusResponse"
                },
                "example": {
                  "temp": "44.4",
                  "cpu": "0.31%",
                  "ram": 287,
                  "stream_state": "active"
                }
              }
            }
          }
        }
      }
    },
    "/api/config": {
      "get": {
        "tags": ["Stream Configuration"],
        "summary": "Get Active Stream Parameters",
        "description": "Retrieves the currently configured video bitrate, target framerate, HUD visibility, color profile, and active hardware encoder.",
        "operationId": "getConfig",
        "responses": {
          "200": {
            "description": "Active stream settings",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ConfigResponse"
                },
                "example": {
                  "bitrate": 3000,
                  "fps": 30,
                  "hud": false,
                  "color_profile": "full",
                  "encoder": "vaapi"
                }
              }
            }
          }
        }
      },
      "post": {
        "tags": ["Stream Configuration"],
        "summary": "Hot-Apply Stream Parameters",
        "description": "Dispatches a hot-apply control packet over UDP port 5001 to dynamically change bitrate, FPS, HUD, or color profile without interrupting the video pipeline.",
        "operationId": "updateConfig",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/ConfigUpdateRequest"
              },
              "example": {
                "bitrate": 3500,
                "fps": 30,
                "hud": false,
                "color": "full"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Parameters accepted and broadcasted to host sender",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/GenericStatusResponse"
                },
                "example": {
                  "status": "ok"
                }
              }
            }
          },
          "400": {
            "description": "Invalid JSON payload"
          }
        }
      }
    },
    "/api/modes": {
      "post": {
        "tags": ["Operating Modes"],
        "summary": "Switch Active Operating Mode",
        "description": "Switches the active display ingress mode: Mode 1 (Network UDP 5000 + Miracast RTSP 7236) or Mode 3 (Direct USB Bulk via FunctionFS).",
        "operationId": "setMode",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/ModeSwitchRequest"
              },
              "example": {
                "mode": "usb-bulk"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Operating mode switched successfully",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/GenericStatusResponse"
                },
                "example": {
                  "status": "switched"
                }
              }
            }
          }
        }
      }
    },
    "/api/mode": {
      "post": {
        "tags": ["Operating Modes"],
        "summary": "Switch Active Operating Mode (Alias)",
        "description": "Alternative endpoint for switching between network and USB Bulk operating modes.",
        "operationId": "setModeAlias",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/ModeSwitchRequest"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Mode switched successfully",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/GenericStatusResponse"
                }
              }
            }
          }
        }
      }
    },
    "/api/stream/start": {
      "post": {
        "tags": ["Stream Playback"],
        "summary": "Resume Stream Decoding",
        "description": "Resumes the hardware V4L2 M2M decoder and HDMI framebuffer display rendering.",
        "operationId": "startStream",
        "responses": {
          "200": {
            "description": "Stream resumed",
            "content": {
              "application/json": {
                "example": {
                  "status": "resumed"
                }
              }
            }
          }
        }
      }
    },
    "/api/stream/stop": {
      "post": {
        "tags": ["Stream Playback"],
        "summary": "Pause Stream Decoding",
        "description": "Pauses video decoding and suspends framebuffer updates to save CPU and power.",
        "operationId": "stopStream",
        "responses": {
          "200": {
            "description": "Stream paused",
            "content": {
              "application/json": {
                "example": {
                  "status": "paused"
                }
              }
            }
          }
        }
      }
    },
    "/api/screenshot": {
      "get": {
        "tags": ["Diagnostics & Logs"],
        "summary": "Capture HDMI Framebuffer Screen",
        "description": "Reads '/dev/fb0' and returns the raw 1280x720 RGB565 framebuffer content for visual remote inspection.",
        "operationId": "captureScreenshot",
        "responses": {
          "200": {
            "description": "Raw 1280x720 RGB565 framebuffer data (1,843,200 bytes)",
            "content": {
              "application/octet-stream": {
                "schema": {
                  "type": "string",
                  "format": "binary"
                }
              }
            }
          },
          "500": {
            "description": "Failed to read /dev/fb0"
          }
        }
      }
    },
    "/api/logs": {
      "get": {
        "tags": ["Diagnostics & Logs"],
        "summary": "Receiver Process Logs",
        "description": "Returns recent standard output and error logs from '/var/log/ext-receiver.log'.",
        "operationId": "getLogs",
        "responses": {
          "200": {
            "description": "Receiver service log text",
            "content": {
              "text/plain; charset=utf-8": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      }
    },
    "/api/dmesg": {
      "get": {
        "tags": ["Diagnostics & Logs"],
        "summary": "Kernel Ring Buffer (dmesg)",
        "description": "Returns the Linux kernel ring buffer log containing driver, USB gadget, and VideoCore IV hardware messages.",
        "operationId": "getDmesg",
        "responses": {
          "200": {
            "description": "Kernel messages",
            "content": {
              "text/plain; charset=utf-8": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      }
    },
    "/api/exec": {
      "post": {
        "tags": ["Diagnostics & Logs"],
        "summary": "Execute Remote Diagnostic Command",
        "description": "Executes an arbitrary shell command on the Raspberry Pi Zero appliance and returns combined stdout and stderr.",
        "operationId": "executeCommand",
        "requestBody": {
          "required": true,
          "content": {
            "text/plain": {
              "schema": {
                "type": "string"
              },
              "example": "vcgencmd measure_temp; free -m; ps aux"
            }
          }
        },
        "responses": {
          "200": {
            "description": "Command execution output",
            "content": {
              "text/plain; charset=utf-8": {
                "schema": {
                  "type": "string"
                }
              }
            }
          },
          "400": {
            "description": "Missing command body"
          }
        }
      }
    },
    "/api/sdcard/status": {
      "get": {
        "tags": ["Micro-SD Storage"],
        "summary": "Micro-SD Card Mount Status",
        "description": "Checks whether the physical Micro-SD boot partition ('/dev/mmcblk0p1') is currently mounted.",
        "operationId": "getSdCardStatus",
        "responses": {
          "200": {
            "description": "Mount state of /mnt/boot",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/SdCardStatusResponse"
                },
                "example": {
                  "mounted": false
                }
              }
            }
          }
        }
      }
    },
    "/api/sdcard/mount": {
      "post": {
        "tags": ["Micro-SD Storage"],
        "summary": "Mount Micro-SD Boot Partition",
        "description": "Mounts '/dev/mmcblk0p1' onto '/mnt/boot' to allow inspecting or modifying appliance files on the physical flash card.",
        "operationId": "mountSdCard",
        "responses": {
          "200": {
            "description": "Partition mounted",
            "content": {
              "application/json": {
                "example": {
                  "status": "mounted"
                }
              }
            }
          }
        }
      }
    },
    "/api/sdcard/unmount": {
      "post": {
        "tags": ["Micro-SD Storage"],
        "summary": "Unmount Micro-SD Boot Partition",
        "description": "Syncs dirty buffers and unmounts '/mnt/boot', ensuring the SD card is in a 100% clean, corruption-free state.",
        "operationId": "unmountSdCard",
        "responses": {
          "200": {
            "description": "Partition unmounted cleanly",
            "content": {
              "application/json": {
                "example": {
                  "status": "unmounted"
                }
              }
            }
          }
        }
      }
    },
    "/api/system/reboot": {
      "post": {
        "tags": ["System & Firmware"],
        "summary": "Reboot Raspberry Pi Zero",
        "description": "Issues a safe hardware reboot to the Raspberry Pi Zero. Boots back into RAM in < 2 seconds.",
        "operationId": "rebootSystem",
        "responses": {
          "200": {
            "description": "Reboot initiated",
            "content": {
              "application/json": {
                "example": {
                  "status": "rebooting"
                }
              }
            }
          }
        }
      }
    },
    "/api/system/update": {
      "post": {
        "tags": ["System & Firmware"],
        "summary": "OTA Firmware/Appliance Update",
        "description": "Downloads a new 'initramfs.cpio.gz' from the given HTTP URL, flashes it directly onto the MicroSD boot partition, and automatically reboots.",
        "operationId": "updateFirmware",
        "requestBody": {
          "required": false,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/UpdateRequest"
              },
              "example": {
                "url": "http://192.168.7.1:8000/initramfs.cpio.gz"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "OTA firmware update initiated in background",
            "content": {
              "application/json": {
                "example": {
                  "status": "updating"
                }
              }
            }
          }
        }
      }
    },
    "/api/service/stop": {
      "post": {
        "tags": ["System & Firmware"],
        "summary": "Stop Receiver Service",
        "description": "Stops the 'ext-receiver' background daemon and cleanly releases DRM KMS / framebuffer resources.",
        "operationId": "stopService",
        "responses": {
          "200": {
            "description": "Service stopped",
            "content": {
              "application/json": {
                "example": {
                  "status": "stopped"
                }
              }
            }
          }
        }
      }
    },
    "/api/network": {
      "get": {
        "tags": ["Networking"],
        "summary": "Get Network Interfaces",
        "description": "Returns current IP and status of network adapters (usb0, wlan0).",
        "operationId": "getNetwork",
        "responses": {
          "200": {
            "description": "Network configuration text",
            "content": {
              "text/plain; charset=utf-8": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      },
      "post": {
        "tags": ["Networking"],
        "summary": "Configure WiFi Settings",
        "description": "Configures wireless SSID and passphrase for standalone Wi-Fi connectivity.",
        "operationId": "setNetwork",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/NetworkConfigRequest"
              },
              "example": {
                "ssid": "MyHomeNetwork",
                "password": "SecretPassword123"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Network configuration applied",
            "content": {
              "application/json": {
                "example": {
                  "status": "configured"
                }
              }
            }
          }
        }
      }
    },
    "/api/media/status": {
      "get": {
        "tags": ["IoT Media & HDMI Visualizer"],
        "summary": "Get Realtime Audio Spectrum & Track Status",
        "description": "Returns current media track info, volume, visualizer status, active video flag, and 24-band live frequency spectrum heights and peaks.",
        "operationId": "getMediaStatus",
        "responses": {
          "200": {
            "description": "Live media and spectrum telemetry",
            "content": {
              "application/json": {
                "example": {
                  "title": "HDMI Digital Audio (48kHz)",
                  "artist": "Host PC Realtime Audio Stream",
                  "album": "PCM / PipeWire Low-Latency",
                  "source": "pc_audio",
                  "state": "playing",
                  "volume": 100,
                  "visualizer_enabled": true,
                  "video_active": false,
                  "audio_active": true,
                  "rms_db": -14.2,
                  "bars": [0.12, 0.45, 0.78, 0.95, 0.82, 0.65, 0.40, 0.35, 0.50, 0.60, 0.72, 0.80, 0.68, 0.55, 0.42, 0.30, 0.25, 0.20, 0.18, 0.15, 0.12, 0.10, 0.08, 0.05],
                  "peaks": [0.20, 0.50, 0.82, 0.98, 0.88, 0.70, 0.45, 0.40, 0.55, 0.65, 0.78, 0.85, 0.72, 0.60, 0.48, 0.35, 0.28, 0.22, 0.20, 0.18, 0.15, 0.12, 0.10, 0.06]
                }
              }
            }
          }
        }
      }
    },
    "/api/media/realtime": {
      "post": {
        "tags": ["IoT Media & HDMI Visualizer"],
        "summary": "Feed Realtime Audio Spectrum Telemetry",
        "description": "Ingests 24 frequency band energies (0.0 to 1.0) and RMS level (dB) calculated by the host or audio engine.",
        "operationId": "feedMediaRealtime",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "example": {
                "bars": [0.25, 0.45, 0.70, 0.85, 0.95, 0.90, 0.75, 0.60, 0.40, 0.35, 0.50, 0.65, 0.80, 0.70, 0.55, 0.40, 0.30, 0.25, 0.20, 0.35, 0.50, 0.40, 0.25, 0.15],
                "rms": -12.5
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Spectrum updated",
            "content": {
              "application/json": {
                "example": {
                  "status": "spectrum_updated"
                }
              }
            }
          }
        }
      }
    },
    "/api/media/visualizer": {
      "post": {
        "tags": ["IoT Media & HDMI Visualizer"],
        "summary": "Enable or Disable HDMI Visualizer",
        "description": "Toggles whether the 30 FPS dynamic audio visualizer is displayed on HDMI when audio is playing and desktop video is idle.",
        "operationId": "setMediaVisualizer",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "example": {
                "enabled": true
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Visualizer state updated",
            "content": {
              "application/json": {
                "example": {
                  "status": "visualizer_updated"
                }
              }
            }
          }
        }
      }
    },
    "/api/media/control": {
      "post": {
        "tags": ["IoT Media & HDMI Visualizer"],
        "summary": "Control Media Playback & Metadata",
        "description": "Controls audio playback state (play/pause/stop) and updates track title, artist, and album.",
        "operationId": "controlMedia",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "example": {
                "action": "play",
                "title": "Song Title",
                "artist": "Artist Name",
                "album": "Album Name"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Media control executed",
            "content": {
              "application/json": {
                "example": {
                  "status": "ok"
                }
              }
            }
          }
        }
      }
    },
    "/api/host/control": {
      "post": {
        "tags": ["Host Remote Control"],
        "summary": "Send Remote Control Command to Host PC",
        "description": "Forwards real-time commands via bidirectional UDP port 5001 to ext-sender on the host PC (start, stop, extend, clone, audio, trigger_hud).",
        "operationId": "sendHostControl",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "example": {
                "action": "extend"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Command forwarded to host",
            "content": {
              "application/json": {
                "example": {
                  "status": "forwarded"
                }
              }
            }
          }
        }
      }
    },
    "/api/host/status": {
      "get": {
        "tags": ["Host Remote Control"],
        "summary": "Get Transmitter Host Connection Status",
        "description": "Returns status of the ext-sender host link and last telemetry ping.",
        "operationId": "getHostStatus",
        "responses": {
          "200": {
            "description": "Host transmitter status",
            "content": {
              "application/json": {
                "example": {
                  "connected": true,
                  "ip": "192.168.7.1",
                  "mode": "extend"
                }
              }
            }
          }
        }
      }
    },
    "/api/bluetooth/discoverable": {
      "post": {
        "tags": ["Audio & Sound"],
        "summary": "Trigger Bluetooth A2DP Pairing Mode",
        "description": "Puts BlueZ bluetooth daemon into discoverable pairing mode for 60 seconds to pair phones and tablets.",
        "operationId": "triggerBluetoothPairing",
        "responses": {
          "200": {
            "description": "Bluetooth pairing activated",
            "content": {
              "application/json": {
                "example": {
                  "status": "pairing_active",
                  "timeout_seconds": 60
                }
              }
            }
          }
        }
      }
    },
    "/api/audio/volume": {
      "post": {
        "tags": ["Audio & Sound"],
        "summary": "Set Digital HDMI Audio Volume",
        "description": "Sets the hardware HDMI ALSA playback volume (0 to 100%).",
        "operationId": "setAudioVolume",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "example": {
                "volume": 85
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Volume updated",
            "content": {
              "application/json": {
                "example": {
                  "volume": 85,
                  "muted": false
                }
              }
            }
          }
        }
      }
    },
    "/api/audio/mute": {
      "post": {
        "tags": ["Audio & Sound"],
        "summary": "Mute or Unmute HDMI Audio",
        "description": "Mutes or unmutes the digital HDMI audio sink.",
        "operationId": "setAudioMute",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "example": {
                "muted": true
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Mute state updated",
            "content": {
              "application/json": {
                "example": {
                  "volume": 85,
                  "muted": true
                }
              }
            }
          }
        }
      }
    },
    "/connect.sh": {
      "get": {
        "tags": ["Client Downloads"],
        "summary": "Quick Connect Bash Script",
        "description": "Fetches the single-command curl startup script for Linux hosts ('curl -s http://192.168.7.2:8080/connect.sh | bash').",
        "operationId": "getConnectScript",
        "responses": {
          "200": {
            "description": "Bash connect script",
            "content": {
              "text/x-shellscript": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      }
    },
    "/download/ext-sender": {
      "get": {
        "tags": ["Client Downloads"],
        "summary": "Download Linux Host Sender Binary",
        "description": "Downloads the precompiled 64-bit Linux 'ext-sender' binary directly from the appliance.",
        "operationId": "downloadSender",
        "responses": {
          "200": {
            "description": "x86_64 ELF binary",
            "content": {
              "application/octet-stream": {
                "schema": {
                  "type": "string",
                  "format": "binary"
                }
              }
            }
          }
        }
      }
    },
    "/download/client.tar.gz": {
      "get": {
        "tags": ["Client Downloads"],
        "summary": "Download Complete Client Toolpack",
        "description": "Downloads the bundled package containing scripts, binaries, udev rules, and the welcome assistant.",
        "operationId": "downloadClientPack",
        "responses": {
          "200": {
            "description": "Tar.gz archive",
            "content": {
              "application/gzip": {
                "schema": {
                  "type": "string",
                  "format": "binary"
                }
              }
            }
          }
        }
      }
    },
    "/download/99-ext-monitor.rules": {
      "get": {
        "tags": ["Client Downloads"],
        "summary": "Download USB Display udev Rules",
        "description": "Downloads Linux udev rules granting unprivileged host access to the Pi Zero USB Bulk display interface.",
        "operationId": "downloadUdevRules",
        "responses": {
          "200": {
            "description": "udev rules file",
            "content": {
              "text/plain": {
                "schema": {
                  "type": "string"
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "StatusResponse": {
        "type": "object",
        "properties": {
          "temp": { "type": "string", "example": "44.4", "description": "Broadcom BCM2835 SoC temperature in °C" },
          "cpu": { "type": "string", "example": "0.31%", "description": "Receiver CPU utilization" },
          "ram": { "type": "integer", "example": 287, "description": "Free system memory in MiB" },
          "stream_state": { "type": "string", "enum": ["active", "paused", "idle"], "example": "active" }
        },
        "required": ["temp", "cpu", "ram", "stream_state"]
      },
      "ConfigResponse": {
        "type": "object",
        "properties": {
          "bitrate": { "type": "integer", "example": 3000, "description": "Bitrate cap in kbps" },
          "fps": { "type": "integer", "example": 30, "description": "Target framerate" },
          "hud": { "type": "boolean", "example": false, "description": "Diagnostic on-screen HUD enabled" },
          "color_profile": { "type": "string", "example": "full", "description": "Active color profile (economy, grayscale, full)" },
          "encoder": { "type": "string", "example": "vaapi", "description": "Hardware encoder API" }
        },
        "required": ["bitrate", "fps", "hud", "color_profile", "encoder"]
      },
      "ConfigUpdateRequest": {
        "type": "object",
        "properties": {
          "bitrate": { "type": "integer", "minimum": 400, "maximum": 8000, "example": 3000 },
          "fps": { "type": "integer", "enum": [15, 24, 30, 45, 60], "example": 30 },
          "hud": { "type": "boolean", "example": false },
          "color": { "type": "string", "enum": ["economy", "grayscale", "full"], "example": "full" }
        }
      },
      "ModeSwitchRequest": {
        "type": "object",
        "properties": {
          "mode": {
            "type": "string",
            "enum": ["usb-bulk", "network-miracast", "miracast-mp2t"],
            "example": "usb-bulk",
            "description": "Target display ingress mode"
          }
        },
        "required": ["mode"]
      },
      "SdCardStatusResponse": {
        "type": "object",
        "properties": {
          "mounted": { "type": "boolean", "example": false, "description": "Whether /mnt/boot is mounted" }
        },
        "required": ["mounted"]
      },
      "UpdateRequest": {
        "type": "object",
        "properties": {
          "url": { "type": "string", "format": "uri", "example": "http://192.168.7.1:8000/initramfs.cpio.gz" }
        }
      },
      "NetworkConfigRequest": {
        "type": "object",
        "properties": {
          "ssid": { "type": "string", "example": "MyWiFi" },
          "password": { "type": "string", "example": "Secret123" }
        },
        "required": ["ssid"]
      },
      "GenericStatusResponse": {
        "type": "object",
        "properties": {
          "status": { "type": "string", "example": "ok" }
        },
        "required": ["status"]
      }
    }
  }
}"##;

pub const SWAGGER_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pi Zero Extended Monitor - Swagger API Explorer</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css" />
    <style>
        :root {
            --bg-color: #0b0f17;
            --surface-color: #121824;
            --accent-cyan: #00e5ff;
            --accent-green: #00ff66;
            --text-color: #e2e8f0;
        }
        body {
            margin: 0;
            padding: 0;
            background-color: var(--bg-color);
            color: var(--text-color);
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
        }
        .header-bar {
            background: linear-gradient(90deg, #101726 0%, #17223b 100%);
            border-bottom: 1px solid rgba(0, 229, 255, 0.25);
            padding: 0.8rem 1.5rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            position: sticky;
            top: 0;
            z-index: 1000;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
        }
        .header-title {
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-weight: 700;
            font-size: 1.1rem;
            color: #fff;
        }
        .header-title span.badge {
            background: rgba(0, 229, 255, 0.15);
            border: 1px solid var(--accent-cyan);
            color: var(--accent-cyan);
            padding: 0.2rem 0.55rem;
            border-radius: 6px;
            font-size: 0.75rem;
            font-weight: 600;
        }
        .header-nav {
            display: flex;
            align-items: center;
            gap: 1rem;
        }
        .header-btn {
            background: rgba(255, 255, 255, 0.08);
            border: 1px solid rgba(255, 255, 255, 0.2);
            color: #fff;
            padding: 0.45rem 0.9rem;
            border-radius: 8px;
            text-decoration: none;
            font-size: 0.85rem;
            font-weight: 600;
            display: inline-flex;
            align-items: center;
            gap: 0.4rem;
            transition: all 0.2s ease;
        }
        .header-btn:hover {
            background: var(--accent-cyan);
            color: #0b0f17;
            border-color: var(--accent-cyan);
        }
        /* Custom dark theme adjustments for Swagger UI */
        .swagger-ui {
            background-color: var(--bg-color);
            color: #cbd5e1;
        }
        .swagger-ui .info {
            margin: 25px 0;
        }
        .swagger-ui .info .title {
            color: #f8fafc;
        }
        .swagger-ui .info p, .swagger-ui .info li {
            color: #94a3b8;
        }
        .swagger-ui .scheme-container {
            background: var(--surface-color);
            box-shadow: none;
            border-bottom: 1px solid rgba(255, 255, 255, 0.08);
            padding: 15px 0;
        }
        .swagger-ui .opblock-tag {
            color: #f1f5f9;
            border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        }
        .swagger-ui .opblock {
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
            margin-bottom: 12px;
        }
        .swagger-ui .opblock .opblock-summary-operation-id,
        .swagger-ui .opblock .opblock-summary-path,
        .swagger-ui .opblock .opblock-summary-description {
            color: #f8fafc;
        }
        .swagger-ui table thead tr th,
        .swagger-ui table thead tr td,
        .swagger-ui .tab li button.tablinks {
            color: #e2e8f0;
        }
        .swagger-ui .opblock-body pre.microlight {
            background: #06090f !important;
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 6px;
            color: #7ee787;
        }
        .swagger-ui input[type=text],
        .swagger-ui textarea {
            background: #06090f !important;
            color: #f1f5f9 !important;
            border: 1px solid rgba(255, 255, 255, 0.2) !important;
            border-radius: 6px !important;
        }
        .swagger-ui select {
            background: #1e293b !important;
            color: #f1f5f9 !important;
            border: 1px solid rgba(255, 255, 255, 0.2) !important;
        }
        .swagger-ui .btn.execute {
            background-color: #00e5ff;
            color: #0b0f17;
            border-color: #00e5ff;
            font-weight: 700;
        }
        .swagger-ui .btn.execute:hover {
            background-color: #00ff66;
            border-color: #00ff66;
        }
        .swagger-ui .btn.cancel {
            background-color: #ef4444;
            color: #fff;
            border-color: #ef4444;
        }
    </style>
</head>
<body>
    <header class="header-bar">
        <div class="header-title">
            <span>📺 Pi Zero Extended Monitor</span>
            <span class="badge" style="background: linear-gradient(135deg, #00e5ff, #7ee787); color: #0b0f17; font-weight: bold;">v2.3.0 OAS 3.0</span>
            <span style="font-size: 0.85rem; color: #94a3b8; font-weight: 400;">Interactive REST API Explorer</span>
        </div>
        <nav class="header-nav">
            <a href="/" class="header-btn">
                <span>📊 Dashboard</span>
            </a>
            <a href="/api/openapi.json" target="_blank" class="header-btn">
                <span>📄 Raw JSON Spec</span>
            </a>
        </nav>
    </header>

    <div id="swagger-ui"></div>

    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            window.ui = SwaggerUIBundle({
                url: "/api/openapi.json",
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "BaseLayout",
                docExpansion: "list",
                filter: true,
                showExtensions: true,
                showCommonExtensions: true,
                defaultModelsExpandDepth: 1,
                displayRequestDuration: true
            });
        };
    </script>
</body>
</html>
"##;
