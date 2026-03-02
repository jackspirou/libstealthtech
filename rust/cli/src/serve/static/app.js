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
    var disconnectBtn = $("#disconnect-btn");
    var deviceList = $("#device-list");

    // ---------- State ----------

    var wsConnection = null;
    var reconnectAttempts = 0;
    var MAX_RECONNECT_DELAY = 30000;
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

    // ---------- Connection state ----------

    var connected = false;

    function isConnected() {
        return connected;
    }

    // ---------- Load initial state ----------

    async function init() {
        try {
            var state = await apiGet("/api/state");
            connected = !!state.connected;
            ST.updateUI(state);
            disconnectBtn.disabled = !connected;

            if (!state.connected) {
                showReconnectOption();
            }
        } catch (e) {
            showReconnectOption();
        }
    }

    function showReconnectOption() {
        var saved = localStorage.getItem(LAST_DEVICE_KEY);
        if (!saved) return;

        try {
            var lastDevice = JSON.parse(saved);
            var label = lastDevice.name || lastDevice.address;
            deviceList.innerHTML =
                '<div class="device-item">' +
                    '<div class="device-info">' +
                        '<span class="device-name">' + label + '</span>' +
                        '<span class="device-address">Last connected device</span>' +
                    '</div>' +
                    '<button class="btn btn-primary btn-connect" id="server-reconnect-btn">Reconnect</button>' +
                '</div>';

            $("#server-reconnect-btn").addEventListener("click", function () {
                autoReconnect(lastDevice.address);
            });
        } catch (e) {
            localStorage.removeItem(LAST_DEVICE_KEY);
        }
    }

    async function autoReconnect(address) {
        deviceList.innerHTML = '<div class="scanning-indicator"><div class="spinner"></div>Scanning and reconnecting...</div>';

        try {
            await apiGet("/api/devices");
            await connectToDevice(address);
        } catch (e) {
            ST.showError("Reconnect failed: " + e.message);
            deviceList.innerHTML = "";
            showReconnectOption();
        }
    }

    // ---------- Scan ----------

    scanBtn.addEventListener("click", async function () {
        scanBtn.disabled = true;
        scanBtn.textContent = "Scanning...";
        deviceList.innerHTML = '<div class="scanning-indicator"><div class="spinner"></div>Scanning for devices...</div>';

        try {
            var devices = await apiGet("/api/devices");
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
            deviceList.innerHTML = "";
        } finally {
            scanBtn.disabled = false;
            scanBtn.textContent = "Scan for Devices";
        }
    });

    // ---------- Connect / Disconnect ----------

    async function connectToDevice(address) {
        var statusDot = $("#status-dot");
        var statusText = $("#status-text");
        var connectionPanel = $("#connection-panel");
        statusDot.className = "status-dot connecting";
        statusText.textContent = "Connecting...";
        if (connectionPanel) connectionPanel.dataset.state = "connecting";

        try {
            var state = await apiPost("/api/connect", { address: address });
            connected = !!state.connected;
            ST.updateUI(state);
            disconnectBtn.disabled = !connected;
            deviceList.innerHTML = "";
            if (state.connected) {
                localStorage.setItem(LAST_DEVICE_KEY, JSON.stringify({
                    address: state.address || address,
                    name: state.name || null,
                }));
            }
            ST.addLogEntry("Connected to " + (state.name || address));
        } catch (e) {
            ST.showError("Connection failed: " + e.message);
            statusDot.className = "status-dot";
            statusText.textContent = "Disconnected";
            if (connectionPanel) connectionPanel.dataset.state = "disconnected";
        }
    }

    async function disconnect() {
        try {
            var state = await apiPost("/api/disconnect", {});
            connected = false;
            ST.updateUI(state);
            disconnectBtn.disabled = true;
            ST.addLogEntry("Disconnected from device");
        } catch (e) {
            ST.showError("Disconnect failed: " + e.message);
        }
    }

    disconnectBtn.addEventListener("click", disconnect);

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
        var delay = Math.min(1000 * Math.pow(2, reconnectAttempts - 1), MAX_RECONNECT_DELAY);
        setTimeout(connectWebSocket, delay);
    }

    connectWebSocket();

    // ---------- Register transport ----------

    ST.registerTransport("server", {
        init: init,
        isConnected: isConnected,
        disconnect: disconnect,
        apiPost: apiPost,
    });
})();
