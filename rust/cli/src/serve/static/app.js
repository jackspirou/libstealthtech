// ============================================================================
// StealthTech Remote -- Server Transport
// REST API + WebSocket connection through the server's btleplug BLE.
// Registers as the "server" transport with shared.js.
// ============================================================================

(function () {
    "use strict";

    var ST = window.StealthTech;
    var $ = ST.$;

    // ---------- DOM references ----------

    var scanBtn = $("#scan-btn");
    var serverReconnectBtn = $("#server-reconnect-btn");
    var serverDividerOr = $("#server-divider-or");
    var deviceList = $("#device-list");
    var connectionPanel = $("#connection-panel");
    var serverConnectingIndicator = $("#server-connecting-indicator");
    var serverConnectingText = $("#server-connecting-text");
    var serverConnectionControls = $("#server-connection-controls");
    var connectedDevice = $("#connected-device");

    // ---------- State ----------

    var wsConnection = null;
    var reconnectAttempts = 0;
    var MAX_RECONNECT_DELAY = 30000;
    var MAX_RECONNECT_ATTEMPTS = 50;
    var LAST_DEVICE_KEY = "stealthtech-last-device";

    // ---------- API helpers ----------

    async function apiGet(path) {
        var res = await fetch(path);
        if (!res.ok) {
            var body = await res.json().catch(function () { return {}; });
            throw new Error(body.error || "Request failed: " + res.status);
        }
        return res.json();
    }

    async function apiPost(path, body) {
        var res = await fetch(path, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        if (!res.ok) {
            var data = await res.json().catch(function () { return {}; });
            throw new Error(data.error || "Request failed: " + res.status);
        }
        return res.json();
    }

    // ---------- Action → endpoint map ----------

    var actionEndpoints = {
        "volume":          "/api/volume",
        "bass":            "/api/bass",
        "treble":          "/api/treble",
        "center-volume":   "/api/center-volume",
        "rear-volume":     "/api/rear-volume",
        "balance":         "/api/balance",
        "power":           "/api/power",
        "mute":            "/api/mute",
        "quietCouch":      "/api/quiet-couch",
        "input":           "/api/input",
        "mode":            "/api/mode",
        "config-shape":    "/api/config-shape",
        "play-pause":      "/api/play-pause",
        "skip":            "/api/skip",
    };

    function send(action, value) {
        var endpoint = actionEndpoints[action];
        if (!endpoint) return Promise.reject(new Error("Unknown action: " + action));
        return apiPost(endpoint, { value: value });
    }

    // ---------- Connection state ----------

    var connected = false;

    function isConnected() {
        return connected;
    }

    // ---------- Card state helpers ----------

    function setServerConnecting(message) {
        if (connectionPanel) connectionPanel.dataset.state = "connecting";
        if (serverConnectingIndicator) serverConnectingIndicator.style.display = "";
        if (serverConnectingText) serverConnectingText.textContent = message || "Connecting...";
        if (serverConnectionControls) serverConnectionControls.style.display = "none";
        deviceList.innerHTML = "";
    }

    function setServerDisconnected() {
        if (connectionPanel) connectionPanel.dataset.state = "disconnected";
        if (serverConnectingIndicator) serverConnectingIndicator.style.display = "none";
        if (serverConnectionControls) serverConnectionControls.style.display = "";
        if (connectedDevice) connectedDevice.style.display = "none";
    }

    // ---------- Load initial state ----------

    async function init() {
        try {
            var state = await apiGet("/api/state");
            connected = !!state.connected;
            ST.updateUI(state);

            if (!state.connected) {
                showReconnectOption();
            }
        } catch (e) {
            showReconnectOption();
        }
    }

    function getLastDevice() {
        var saved = localStorage.getItem(LAST_DEVICE_KEY);
        if (!saved) return null;
        try {
            return JSON.parse(saved);
        } catch (e) {
            localStorage.removeItem(LAST_DEVICE_KEY);
            return null;
        }
    }

    function showReconnectOption() {
        var lastDevice = getLastDevice();
        if (!lastDevice) return;

        var label = lastDevice.name || lastDevice.address;
        serverReconnectBtn.textContent = "Reconnect to " + label;
        serverReconnectBtn.style.display = "";
        serverDividerOr.style.display = "";
    }

    async function autoReconnect() {
        var lastDevice = getLastDevice();
        if (!lastDevice) return;

        setServerConnecting("Reconnecting to " + (lastDevice.name || lastDevice.address) + "...");

        try {
            await apiGet("/api/devices");
            await connectToDevice(lastDevice.address);
        } catch (e) {
            ST.showError("Reconnect failed: " + e.message);
            setServerDisconnected();
            showReconnectOption();
        }
    }

    serverReconnectBtn.addEventListener("click", autoReconnect);

    // ---------- Scan ----------

    scanBtn.addEventListener("click", async function () {
        scanBtn.disabled = true;
        setServerConnecting("Scanning for devices...");

        try {
            var devices = await apiGet("/api/devices");

            // Restore controls and show results
            if (serverConnectingIndicator) serverConnectingIndicator.style.display = "none";
            if (serverConnectionControls) serverConnectionControls.style.display = "";
            if (connectionPanel) connectionPanel.dataset.state = "disconnected";

            deviceList.innerHTML = "";

            if (devices.length === 0) {
                deviceList.innerHTML = '<div class="scanning-indicator">No devices found. Make sure your center channel is powered on.</div>';
            } else {
                devices.forEach(function (device) {
                    var item = document.createElement("div");
                    item.className = "device-item";
                    item.innerHTML =
                        '<div class="device-info">' +
                            '<span class="device-name">' + (device.name || "Unknown Device") + '</span>' +
                            '<span class="device-address">' + device.address + '</span>' +
                        '</div>' +
                        '<div style="display:flex;align-items:center;gap:10px">' +
                            '<span class="device-rssi">' + (device.rssi != null ? device.rssi + " dBm" : "") + '</span>' +
                            '<button class="btn btn-primary btn-connect">Connect</button>' +
                        '</div>';

                    item.querySelector(".btn-connect").addEventListener("click", function () {
                        connectToDevice(device.address);
                    });

                    deviceList.appendChild(item);
                });
            }
        } catch (e) {
            ST.showError("Scan failed: " + e.message);
            setServerDisconnected();
        } finally {
            scanBtn.disabled = false;
        }
    });

    // ---------- Connect / Disconnect ----------

    async function connectToDevice(address) {
        setServerConnecting("Connecting...");

        try {
            var state = await apiPost("/api/connect", { address: address });
            connected = !!state.connected;
            ST.updateUI(state);
            deviceList.innerHTML = "";
            if (state.connected) {
                localStorage.setItem(LAST_DEVICE_KEY, JSON.stringify({
                    address: state.address || address,
                    name: state.name || null,
                }));
                // Set up reconnect for next disconnect
                serverReconnectBtn.textContent = "Reconnect to " + (state.name || address);
                serverReconnectBtn.style.display = "";
                serverDividerOr.style.display = "";
            }
            ST.addLogEntry("Connected to " + (state.name || address));
        } catch (e) {
            ST.showError("Connection failed: " + e.message);
            setServerDisconnected();
            showReconnectOption();
        }
    }

    async function disconnect() {
        try {
            var state = await apiPost("/api/disconnect", {});
            connected = false;
            ST.updateUI(state);
            ST.addLogEntry("Disconnected from device");
        } catch (e) {
            ST.showError("Disconnect failed: " + e.message);
        }
    }

    // ---------- WebSocket ----------

    function connectWebSocket() {
        var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        var url = protocol + "//" + window.location.host + "/ws";

        wsConnection = new WebSocket(url);

        wsConnection.onopen = function () {
            reconnectAttempts = 0;
            ST.addLogEntry("[WS] Connected");
        };

        wsConnection.onmessage = function (event) {
            try {
                var data = JSON.parse(event.data);
                ST.addLogEntry(data.decoded || event.data);
            } catch (e) {
                ST.addLogEntry(event.data);
            }
        };

        wsConnection.onclose = function () {
            ST.addLogEntry("[WS] Disconnected");
            scheduleReconnect();
        };

        wsConnection.onerror = function () {
            // onclose will fire after this
        };
    }

    function scheduleReconnect() {
        reconnectAttempts++;
        if (reconnectAttempts > MAX_RECONNECT_ATTEMPTS) {
            ST.addLogEntry("[WS] Max reconnect attempts reached (" + MAX_RECONNECT_ATTEMPTS + "), giving up");
            return;
        }
        var base = 2000 * Math.pow(2, reconnectAttempts - 1);
        var jitter = Math.round(Math.random() * 1000);
        var delay = Math.min(base + jitter, MAX_RECONNECT_DELAY);
        setTimeout(connectWebSocket, delay);
    }

    connectWebSocket();

    // ---------- Register transport ----------

    ST.registerTransport("server", {
        init: init,
        isConnected: isConnected,
        disconnect: disconnect,
        send: send,
    });
})();
