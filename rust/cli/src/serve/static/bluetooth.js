// ============================================================================
// StealthTech Remote -- Web Bluetooth Transport
// ES module using WASM protocol bindings + navigator.bluetooth.
// Registers as the "bluetooth" transport with shared.js.
// ============================================================================

import init, * as lib from "./pkg/libstealthtech_wasm.js";

await init();

// ---------- WASM constants ----------

const SERVICE_UUID = lib.service_uuid();
const UPSTREAM_UUID = lib.upstream_char_uuid();
const state = new lib.WasmDeviceState();

// ---------- DOM references ----------

const ST = window.StealthTech;
const $ = ST.$;

const connectBtn = $("#connect-btn");
const btSavedDevice = $("#bt-saved-device");
const btScanDivider = $("#bt-scan-divider");
const compatBanner = $("#compat-banner");
const connectionPanel = $("#connection-panel");
const connectingIndicator = $("#connecting-indicator");
const connectingText = $("#connecting-text");
const connectionControls = $("#connection-controls");
const statusDot = $("#status-dot");
const statusText = $("#status-text");

const BT_DEVICE_KEY = "stealthtech-last-bt-device";
const DEVICE_NAME_RE = /stealthtech|lovesac|sound.*charge|hk_lovesac|ee4034/i;

function getLastBtDevice() {
    var raw = localStorage.getItem(BT_DEVICE_KEY);
    if (!raw) return null;
    try {
        var obj = JSON.parse(raw);
        return typeof obj === "object" && obj !== null ? obj : { name: raw };
    } catch (e) {
        return { name: raw };
    }
}

// ---------- BLE state ----------

let bleDevice = null;
let bleServer = null;
let bleService = null;
let charCache = {};
let bleConnected = false;
let upstreamChar = null; // tracked for listener cleanup
let lastStoredFirmware = null;
let lastStoredSubwoofer = null;
let _intentionalDisconnect = false;

// ---------- Browser compat gate ----------

function checkBrowserCompat() {
    const ua = navigator.userAgent;
    const isChrome = /Chrome\/(\d+)/.test(ua) && !/Edg\//.test(ua);
    const isEdge = /Edg\/(\d+)/.test(ua);
    const isOpera = /OPR\//.test(ua);
    const isSamsung = /SamsungBrowser\//.test(ua);
    const isSafari = /Safari\//.test(ua) && !isChrome && !isEdge && !isOpera;
    const isFirefox = /Firefox\//.test(ua);
    const isIOS = /iPad|iPhone|iPod/.test(ua) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1);
    const isAndroid = /Android/.test(ua);

    // Web Bluetooth available — no banner needed
    if (navigator.bluetooth) return;

    connectBtn.disabled = true;
    btSavedDevice.style.display = "none";

    var msg = "<p><strong>Web Bluetooth is not available</strong> in this browser.</p>";

    if (isIOS) {
        msg += "<p>Web Bluetooth is not supported on iOS. " +
            'Try <a href="https://apps.apple.com/app/bluefy-web-ble-browser/id1492822055" target="_blank" rel="noopener">Bluefy</a>, ' +
            "a browser with Web Bluetooth support, or use the Server BLE mode from a desktop.</p>";
    } else if (isFirefox) {
        msg += '<p>Firefox does not support Web Bluetooth. ' +
            '<a href="https://www.google.com/chrome/" target="_blank" rel="noopener">Download Chrome</a> ' +
            "or use a Chromium-based browser (Edge, Opera).</p>";
    } else if (isSafari) {
        msg += '<p>Safari does not support Web Bluetooth. ' +
            '<a href="https://www.google.com/chrome/" target="_blank" rel="noopener">Download Chrome</a> ' +
            "or use a Chromium-based browser (Edge, Opera).</p>";
    } else if (isChrome) {
        var match = ua.match(/Chrome\/(\d+)/);
        var version = match ? parseInt(match[1], 10) : 0;
        msg += "<p>Your Chrome version (" + version + ") may be too old. " +
            'Please <a href="chrome://settings/help" target="_blank" rel="noopener">update Chrome</a> to version 56 or later.</p>';
    } else if (isAndroid && isSamsung) {
        msg += '<p>Samsung Internet does not support Web Bluetooth. ' +
            '<a href="https://www.google.com/chrome/" target="_blank" rel="noopener">Open in Chrome</a> instead.</p>';
    } else {
        msg += '<p><a href="https://www.google.com/chrome/" target="_blank" rel="noopener">Download Chrome</a> ' +
            "or use a Chromium-based browser (Edge, Opera) that supports Web Bluetooth.</p>";
    }

    compatBanner.innerHTML = msg;
    compatBanner.style.display = "block";
}

checkBrowserCompat();

// ---------- Auto-reconnect previously paired devices ----------

function showSavedDevice(device) {
    if (typeof device === "string") device = { name: device };
    ST.renderSavedDevice(btSavedDevice, {
        name: device.name,
        firmware: device.firmware,
        subwoofer: device.subwoofer,
        orEl: btScanDivider,
        controlsEl: connectionControls,
        onReconnect: reconnect,
        onForget: function () {
            localStorage.removeItem(BT_DEVICE_KEY);
            bleDevice = null;
            btSavedDevice.style.display = "none";
            btSavedDevice.innerHTML = "";
            if (btScanDivider) btScanDivider.style.display = "none";
            if (connectionControls) connectionControls.classList.remove("slim");
        },
    });
}

async function checkPreviousDevices() {
    // Try the Web Bluetooth getDevices() API first
    if (navigator.bluetooth && navigator.bluetooth.getDevices) {
        try {
            const devices = await navigator.bluetooth.getDevices();
            const prev = devices.find((d) => d.name && DEVICE_NAME_RE.test(d.name));
            if (prev) {
                bleDevice = prev;
                bleDevice.addEventListener("gattserverdisconnected", onDisconnect);
                var stored = getLastBtDevice() || {};
                stored.name = prev.name;
                showSavedDevice(stored);
                return;
            }
        } catch (e) {
            // getDevices() not supported or no permissions -- ignore
        }
    }

    // Fall back to localStorage for browsers where getDevices() is unavailable
    const saved = getLastBtDevice();
    if (saved) {
        showSavedDevice(saved);
    }
}

// ---------- Action → BLE command maps ----------

const sliderCommands = {
    "volume":          "SetVolume",
    "bass":            "SetBass",
    "treble":          "SetTreble",
    "center-volume":   "SetCenterVolume",
    "rear-volume":     "SetRearChannelVolume",
    "balance":         "SetBalance",
};

const toggleCommands = {
    "power":      "SetPower",
    "mute":       "SetMute",
    "quietCouch": "SetQuietCouch",
};

const inputEnum = {
    hdmi:      "HdmiArc",
    bluetooth: "Bluetooth",
    aux:       "Aux",
    optical:   "Optical",
};

const modeEnum = {
    movies: "Movies",
    music:  "Music",
    tv:     "Tv",
    news:   "News",
    manual: "Manual",
};

const shapeEnum = {
    straight: "Straight",
    lshape:   "LShape",
    ushape:   "UShape",
    pit:      "Pit",
};

// ---------- BLE GATT setup ----------

async function setupGatt() {
    bleServer = await bleDevice.gatt.connect();
    bleService = await bleServer.getPrimaryService(SERVICE_UUID);
    charCache = {};

    if (upstreamChar) {
        upstreamChar.removeEventListener("characteristicvaluechanged", onNotification);
    }
    upstreamChar = await bleService.getCharacteristic(UPSTREAM_UUID);
    await upstreamChar.startNotifications();
    upstreamChar.addEventListener("characteristicvaluechanged", onNotification);
}

async function requestInitialState() {
    await sendCommand('"GetState"');
    await sendCommand('"GetFirmwareVersion"');
}

// ---------- BLE write queue ----------

let writeQueue = Promise.resolve();

function sendCommand(cmdJson) {
    const result = writeQueue.then(async () => {
        const encoded = JSON.parse(lib.encode_command(cmdJson));
        if (!charCache[encoded.uuid]) {
            charCache[encoded.uuid] = await bleService.getCharacteristic(encoded.uuid);
        }
        await charCache[encoded.uuid].writeValueWithoutResponse(new Uint8Array(encoded.data));
    });
    // Keep the queue going even if one command fails
    writeQueue = result.catch(() => {});
    return result;
}

function send(action, value) {
    let cmd;
    if (sliderCommands[action]) {
        cmd = '{"' + sliderCommands[action] + '": ' + value + '}';
    } else if (toggleCommands[action]) {
        cmd = '{"' + toggleCommands[action] + '": ' + value + '}';
    } else if (action === "input") {
        cmd = '{"SetInput": "' + inputEnum[value] + '"}';
    } else if (action === "mode") {
        cmd = '{"SetSoundMode": "' + modeEnum[value] + '"}';
    } else if (action === "config-shape") {
        cmd = '{"SetConfigShape": "' + shapeEnum[value] + '"}';
    } else if (action === "play-pause") {
        cmd = '{"SetPlayPause": ' + value + '}';
    } else if (action === "skip") {
        cmd = '{"SetSkip": ' + value + '}';
    } else {
        return Promise.reject(new Error("Unknown action: " + action));
    }
    return sendCommand(cmd);
}

// ---------- Card state helpers ----------

function setStatusDisconnected() {
    statusDot.className = "status-dot";
    statusText.textContent = "Disconnected";
}

function setCardConnecting(message) {
    if (connectionPanel) connectionPanel.dataset.state = "connecting";
    if (connectingIndicator) connectingIndicator.style.display = "";
    if (connectingText) connectingText.textContent = message || "Connecting...";
    if (connectionControls) connectionControls.style.display = "none";
    if (btSavedDevice) btSavedDevice.style.display = "none";
    if (btScanDivider) btScanDivider.style.display = "none";
}

function setCardDisconnected() {
    if (connectionPanel) connectionPanel.dataset.state = "disconnected";
    if (connectingIndicator) connectingIndicator.style.display = "none";
    if (connectionControls) connectionControls.style.display = "";
}

// ---------- Connect ----------

async function connect() {
    try {
        statusDot.className = "status-dot connecting";
        statusText.textContent = "Connecting...";
        setCardConnecting("Connecting...");

        bleDevice = await navigator.bluetooth.requestDevice({
            filters: [
                { namePrefix: "StealthTech" },
                { namePrefix: "stealthtech" },
                { namePrefix: "Stealth Tech" },
                { namePrefix: "Lovesac" },
                { namePrefix: "lovesac" },
                { namePrefix: "Sound + Charge" },
                { namePrefix: "Sound+Charge" },
                { namePrefix: "HK_Lovesac" },
                { namePrefix: "hk_lovesac" },
                { namePrefix: "EE4034" },
                { namePrefix: "ee4034" },
            ],
            optionalServices: [SERVICE_UUID],
        });
        bleDevice.addEventListener("gattserverdisconnected", onDisconnect);

        await setupGatt();
        onConnected();
        await requestInitialState();
    } catch (e) {
        if (e.name === "NotFoundError") {
            setStatusDisconnected();
            setCardDisconnected();
            return;
        }
        ST.showError("Connection failed: " + e.message);
        setStatusDisconnected();
        setCardDisconnected();
    }
}

async function reconnect() {
    // Try to recover bleDevice via getDevices() if we only have localStorage
    if (!bleDevice || !bleDevice.gatt) {
        if (navigator.bluetooth && navigator.bluetooth.getDevices) {
            try {
                const devices = await navigator.bluetooth.getDevices();
                const prev = devices.find((d) => d.name && DEVICE_NAME_RE.test(d.name));
                if (prev) {
                    bleDevice = prev;
                    bleDevice.addEventListener("gattserverdisconnected", onDisconnect);
                }
            } catch (e) {
                // getDevices() unavailable — fall through
            }
        }
        if (!bleDevice || !bleDevice.gatt) {
            return connect();
        }
    }

    try {
        statusDot.className = "status-dot connecting";
        statusText.textContent = "Reconnecting...";
        setCardConnecting("Reconnecting to " + (bleDevice.name || "device") + "...");

        await setupGatt();
        onConnected();
        await requestInitialState();
    } catch (e) {
        ST.showError("Reconnect failed: " + e.message);
        setStatusDisconnected();
        setCardDisconnected();
        showSavedDevice(getLastBtDevice() || { name: bleDevice ? bleDevice.name || "device" : "device" });
    }
}

function onConnected() {
    bleConnected = true;

    // Persist device as JSON for reconnect across refresh
    var existing = getLastBtDevice() || {};
    existing.name = bleDevice.name || "";
    localStorage.setItem(BT_DEVICE_KEY, JSON.stringify(existing));

    // Prepare saved device row for next disconnect
    showSavedDevice(existing);

    if (ST.isActiveTransport("bluetooth")) {
        ST.updateUI({ connected: true, name: bleDevice.name || null });
    }
    ST.addLogEntry("Connected to " + (bleDevice.name || "device"));
}

function onDisconnect() {
    bleConnected = false;
    charCache = {};

    if (_intentionalDisconnect) {
        _intentionalDisconnect = false;
        if (ST.isActiveTransport("bluetooth")) {
            ST.updateUI({ connected: false });
        }
        var saved = getLastBtDevice();
        if (saved) showSavedDevice(saved);
        ST.addLogEntry("Device disconnected");
        return;
    }

    // Unexpected disconnect — auto-reconnect
    ST.addLogEntry("Connection lost, reconnecting...");
    setCardConnecting("Reconnecting...");
    statusDot.className = "status-dot connecting";
    statusText.textContent = "Reconnecting...";
    autoReconnect(0);
}

async function disconnect() {
    _intentionalDisconnect = true;
    if (bleDevice && bleDevice.gatt.connected) {
        bleDevice.gatt.disconnect();
        // onDisconnect handler will fire via gattserverdisconnected event
        return;
    }
    // Fallback cleanup (no gattserverdisconnected event will fire)
    bleConnected = false;
    charCache = {};
    _intentionalDisconnect = false;
    if (ST.isActiveTransport("bluetooth")) {
        ST.updateUI({ connected: false });
    }
    var saved = getLastBtDevice();
    if (saved) showSavedDevice(saved);
    ST.addLogEntry("Disconnected from device");
}

// ---------- Auto-reconnect on unexpected disconnect ----------

var MAX_AUTO_RECONNECT = 3;
var AUTO_RECONNECT_DELAYS = [1500, 3000, 5000];

function autoReconnect(attempt) {
    if (attempt >= MAX_AUTO_RECONNECT) {
        if (ST.isActiveTransport("bluetooth")) {
            ST.updateUI({ connected: false });
        }
        var saved = getLastBtDevice();
        if (saved) showSavedDevice(saved);
        ST.addLogEntry("Reconnect failed after " + MAX_AUTO_RECONNECT + " attempts");
        return;
    }

    var delay = AUTO_RECONNECT_DELAYS[attempt] || 5000;
    setTimeout(async function () {
        try {
            await setupGatt();
            onConnected();
            await requestInitialState();
        } catch (e) {
            ST.addLogEntry("Reconnect attempt " + (attempt + 1) + " failed");
            autoReconnect(attempt + 1);
        }
    }, delay);
}

connectBtn.addEventListener("click", connect);

// ---------- Notification handler ----------

function onNotification(event) {
    const value = new Uint8Array(event.target.value.buffer);
    try {
        const decoded = lib.decode_response(UPSTREAM_UUID, value);
        state.apply_response(decoded);

        // Build a state object from the WASM state for the shared updateUI
        const fwJson = state.firmware_status();
        const uiState = {
            volume: state.volume(),
            bass: state.bass(),
            treble: state.treble(),
            center_volume: state.center_volume(),
            rear_channel_volume: state.rear_channel_volume(),
            balance: state.balance(),
            power: state.power(),
            mute: state.mute(),
            quiet_couch: state.quiet_couch(),
            input: state.input(),
            sound_mode: state.sound_mode(),
            config_shape: state.config_shape(),
            subwoofer_connected: state.subwoofer_connected(),
            firmware: fwJson ? JSON.parse(fwJson) : null,
        };
        if (ST.isActiveTransport("bluetooth")) {
            ST.updateUI(uiState);
        }

        // Update stored device details only when firmware/subwoofer actually change
        var fwStr = uiState.firmware ? ST.buildFirmwareString(uiState.firmware) : null;
        var subVal = uiState.subwoofer_connected;
        if ((fwStr && fwStr !== lastStoredFirmware) || (subVal != null && subVal !== lastStoredSubwoofer)) {
            var stored = getLastBtDevice();
            if (stored) {
                if (fwStr) { stored.firmware = fwStr; lastStoredFirmware = fwStr; }
                if (subVal != null) { stored.subwoofer = subVal; lastStoredSubwoofer = subVal; }
                localStorage.setItem(BT_DEVICE_KEY, JSON.stringify(stored));
            }
        }

        ST.addLogEntry(decoded);
    } catch (e) {
        ST.addLogEntry("Decode error: " + e.message);
    }
}

// ---------- Init ----------

let btInitDone = false;

function btInit() {
    if (btInitDone) return;
    btInitDone = true;
    checkPreviousDevices();
}

// ---------- Register transport ----------

ST.registerTransport("bluetooth", {
    init: btInit,
    isConnected: function () { return bleConnected; },
    disconnect: disconnect,
    send: send,
});
