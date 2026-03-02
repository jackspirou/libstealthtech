// ============================================================================
// StealthTech Remote -- Shared Module
// Theme, toast, log, mode switching, transport abstraction,
// and unified control handlers.
// ============================================================================

(function () {
    "use strict";

    // ---------- DOM helpers ----------

    var $ = function (sel) { return document.querySelector(sel); };
    var $$ = function (sel) { return document.querySelectorAll(sel); };

    // ---------- DOM references ----------

    var statusDot = $("#status-dot");
    var statusText = $("#status-text");
    var logContainer = $("#log-container");
    var logEmpty = $("#log-empty");
    var clearLogBtn = $("#clear-log-btn");
    var themeToggle = $("#theme-toggle");
    var themeIcon = $("#theme-icon");

    // Connection card elements (used by updateUI)
    var connectionPanel = $("#connection-panel");
    var connectedDevice = $("#connected-device");
    var connDeviceName = $("#connected-device-name");
    var connFwText = $("#connected-firmware-text");
    var fwBanner = $("#firmware-update-banner");
    var btConnectingIndicator = $("#connecting-indicator");
    var btConnectionControls = $("#connection-controls");
    var srvConnectingIndicator = $("#server-connecting-indicator");
    var srvConnectionControls = $("#server-connection-controls");
    var subRow = $("#subwoofer-row");
    var subStatus = $("#subwoofer-status");
    var mediaCard = $("#media-card");

    // ---------- Percentage display helper ----------

    function toPercent(value, max) {
        if (!max) return "0%";
        return Math.round((value / max) * 100) + "%";
    }

    // Sliders
    var sliders = {
        volume:          { el: $("#volume-slider"),        val: $("#volume-value") },
        bass:            { el: $("#bass-slider"),           val: $("#bass-value") },
        treble:          { el: $("#treble-slider"),         val: $("#treble-value") },
        "center-volume": { el: $("#center-volume-slider"), val: $("#center-volume-value") },
        "rear-volume":   { el: $("#rear-volume-slider"),   val: $("#rear-volume-value") },
        balance:         { el: $("#balance-slider"),        val: $("#balance-value") },
    };

    // Toggle buttons
    var toggles = {
        power:      { el: $("#power-btn") },
        mute:       { el: $("#mute-btn") },
        quietCouch: { el: $("#quiet-couch-btn") },
        surround: { el: $("#surround-btn") },
    };

    // ---------- Theme ----------

    var THEME_KEY = "stealthtech-theme";
    var themes = ["auto", "light", "dark"];
    var currentThemeIndex = 0;

    function initTheme() {
        var saved = localStorage.getItem(THEME_KEY);
        if (saved) {
            currentThemeIndex = themes.indexOf(saved);
            if (currentThemeIndex < 0) currentThemeIndex = 0;
        }
        applyTheme();
    }

    function applyTheme() {
        var theme = themes[currentThemeIndex];
        var root = document.documentElement;

        if (theme === "auto") {
            root.removeAttribute("data-theme");
        } else {
            root.setAttribute("data-theme", theme);
        }

        themeIcon.textContent = theme === "auto" ? "\u2699" : theme === "light" ? "\u2600" : "\u263E";
        localStorage.setItem(THEME_KEY, theme);
    }

    themeToggle.addEventListener("click", function () {
        currentThemeIndex = (currentThemeIndex + 1) % themes.length;
        applyTheme();
    });

    initTheme();

    // ---------- Error toast ----------

    var toastEl = null;
    var toastTimeout = null;

    function showError(message) {
        if (!toastEl) {
            toastEl = document.createElement("div");
            toastEl.className = "error-toast";
            toastEl.setAttribute("role", "alert");
            document.body.appendChild(toastEl);
        }

        toastEl.textContent = message;
        toastEl.classList.add("visible");

        clearTimeout(toastTimeout);
        toastTimeout = setTimeout(function () {
            toastEl.classList.remove("visible");
        }, 4000);
    }

    // ---------- Notification log ----------

    var fwTypeNames = { 1: "MCU", 2: "DSP", 3: "EQ" };

    var logFormatters = {
        Volume:             function (v) { return "Volume: " + v + "/36"; },
        CenterVolume:       function (v) { return "Center: " + v + "/30"; },
        Treble:             function (v) { return "Treble: " + v + "/20"; },
        Bass:               function (v) { return "Bass: " + v + "/20"; },
        MuteState:          function (v) { return "Mute: " + (v ? "on" : "off"); },
        QuietMode:          function (v) { return "Quiet Mode: " + (v ? "on" : "off"); },
        Balance:            function (v) { return "Balance: " + v + "/100"; },
        Layout:             function (v) { return "Layout: " + v; },
        CurrentInput:       function (v) { return "Input: " + v; },
        Power:              function (v) { return "Power: " + (v ? "on" : "standby"); },
        CurrentSoundMode:   function (v) { return "Sound Mode: " + v; },
        Covering:           function (v) { return "Covering: " + v; },
        ArmType:            function (v) { return "Arm Type: " + v; },
        SubwooferConnected: function (v) { return "Subwoofer: " + (v ? "connected" : "disconnected"); },
        RearVolume:         function (v) { return "Rear: " + v + "/30"; },
        FirmwareVersion:    function (v) {
            var name = fwTypeNames[v.fw_type] || "Type " + v.fw_type;
            return "Firmware " + name + ": v" + v.major + "." + v.minor;
        },
    };

    function formatLogMessage(message) {
        var parsed;
        try { parsed = JSON.parse(message); } catch (e) { return message; }
        if (typeof parsed !== "object" || parsed === null) return message;
        var keys = Object.keys(parsed);
        if (keys.length !== 1) return message;
        var fmt = logFormatters[keys[0]];
        return fmt ? fmt(parsed[keys[0]]) : message;
    }

    function addLogEntry(message) {
        if (logEmpty) {
            logEmpty.style.display = "none";
        }

        var entry = document.createElement("div");
        entry.className = "log-entry";

        var now = new Date();
        var time = now.toLocaleTimeString("en-US", { hour12: true }) +
            "." + String(now.getMilliseconds()).padStart(3, "0");

        entry.innerHTML = '<span class="log-time">' + time + '</span><span class="log-message">' + escapeHtml(formatLogMessage(message)) + '</span>';

        logContainer.appendChild(entry);
        logContainer.scrollTop = logContainer.scrollHeight;

        // Keep log to 200 entries max
        while (logContainer.children.length > 201) {
            logContainer.removeChild(logContainer.children[1]);
        }
    }

    function escapeHtml(text) {
        return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    }

    clearLogBtn.addEventListener("click", function () {
        logContainer.innerHTML = '<div class="log-empty" id="log-empty">No notifications yet. Connect to a device to begin.</div>';
    });

    // ---------- Transport abstraction ----------

    var transports = {};
    var activeMode = null; // set after transports register
    var transportResolved = false;

    function registerTransport(name, transport) {
        transports[name] = transport;
        // If the initial timer already fired (e.g. slow WASM load), re-resolve now
        if (transportResolved) {
            resolveTransports();
        }
    }

    function getActiveTransport() {
        return transports[activeMode] || null;
    }

    // ---------- Mode tab switching ----------

    var modeTabs = $$(".mode-tab");
    var modeTabsContainer = $(".mode-tabs");
    var serverConnection = $("#server-connection");
    var bluetoothConnection = $("#bluetooth-connection");

    function switchMode(mode) {
        var changing = mode !== activeMode;

        // If switching away from a connected transport, disconnect first
        if (changing) {
            var currentTransport = getActiveTransport();
            if (currentTransport && currentTransport.isConnected && currentTransport.isConnected()) {
                currentTransport.disconnect();
            }
        }

        activeMode = mode;
        localStorage.setItem("stealthtech-mode", mode);

        // Update tab visuals
        modeTabs.forEach(function (tab) {
            tab.classList.toggle("active", tab.dataset.mode === mode);
        });

        // Show/hide connection panels
        if (serverConnection) serverConnection.style.display = mode === "server" ? "" : "none";
        if (bluetoothConnection) bluetoothConnection.style.display = mode === "bluetooth" ? "" : "none";

        // Initialize the transport on first switch to this mode
        if (changing) {
            var newTransport = getActiveTransport();
            if (newTransport && newTransport.init) {
                newTransport.init();
            }
        }
    }

    modeTabs.forEach(function (tab) {
        tab.addEventListener("click", function () {
            switchMode(tab.dataset.mode);
        });
    });

    // ---------- Auto-detect available transports ----------
    // Wait briefly for scripts to register, then decide mode and tab visibility.

    function resolveTransports() {
        transportResolved = true;
        var names = Object.keys(transports);
        var saved = localStorage.getItem("stealthtech-mode");

        // Show or hide tabs based on how many transports registered
        if (modeTabsContainer) {
            modeTabsContainer.style.display = names.length <= 1 ? "none" : "";
        }

        // Pick mode: prefer saved choice if that transport exists,
        // otherwise prefer "server" (CLI context), fall back to whatever registered.
        var mode;
        if (saved && transports[saved]) {
            mode = saved;
        } else if (transports["server"]) {
            mode = "server";
        } else if (names.length > 0) {
            mode = names[0];
        } else {
            mode = "server"; // nothing registered yet, show server panel as default
        }

        switchMode(mode);
    }

    // Short delay so both <script src="app.js"> and <script type="module" src="bluetooth.js"> can register
    setTimeout(resolveTransports, 120);

    // ---------- Controls enabled/disabled ----------

    function setControlsEnabled(enabled) {
        // Sliders
        Object.keys(sliders).forEach(function (key) {
            sliders[key].el.disabled = !enabled;
        });
        // Toggles
        Object.keys(toggles).forEach(function (key) {
            toggles[key].el.disabled = !enabled;
        });
        // Input source buttons
        $$("[data-input]").forEach(function (btn) {
            btn.disabled = !enabled;
        });
        // Sound mode buttons (exclude mode-tab buttons which have data-mode for tab switching)
        $$(".btn-option[data-mode]").forEach(function (btn) {
            btn.disabled = !enabled;
        });
        // Shape buttons
        $$("[data-shape]").forEach(function (btn) {
            btn.disabled = !enabled;
        });
        // Media control buttons
        var mediaBtn = $("#play-pause-btn");
        if (mediaBtn) mediaBtn.disabled = !enabled;
        var skipFwd = $("#skip-fwd-btn");
        if (skipFwd) skipFwd.disabled = !enabled;
        var skipBack = $("#skip-back-btn");
        if (skipBack) skipBack.disabled = !enabled;
    }

    // Start with controls disabled
    setControlsEnabled(false);

    // ---------- UI state updates ----------

    // Normalize server API format strings to match WASM/protocol enum values
    var inputNormalize = {
        "HDMI ARC": "HdmiArc",
        "Bluetooth": "Bluetooth",
        "AUX": "Aux",
        "Optical": "Optical",
        // Already-normalized values pass through
        "HdmiArc": "HdmiArc",
    };

    var modeNormalize = {
        "TV": "Tv",
        "Movies": "Movies",
        "Music": "Music",
        "News": "News",
        "Manual": "Manual",
        // Already-normalized values pass through
        "Tv": "Tv",
    };

    // Mapping from data-* attribute values to normalized enum values
    var inputMap = {
        hdmi: "HdmiArc",
        bluetooth: "Bluetooth",
        aux: "Aux",
        optical: "Optical",
    };

    var modeMap = {
        movies: "Movies",
        music: "Music",
        tv: "Tv",
        news: "News",
        manual: "Manual",
    };

    var shapeNormalize = {
        "Straight": "Straight",
        "L-Shape": "LShape",
        "U-Shape": "UShape",
        "Pit": "Pit",
        "LShape": "LShape",
        "UShape": "UShape",
    };

    var shapeMap = {
        straight: "Straight",
        lshape: "LShape",
        ushape: "UShape",
        pit: "Pit",
    };

    function updateUI(state) {
        if (!state) return;

        // Connection status
        if (state.connected != null) {
            if (state.connected) {
                statusDot.className = "status-dot connected";
                statusText.innerHTML = "Connected" + (state.name ? ' <span class="status-device-name">- ' + escapeHtml(state.name) + '</span>' : "");
                setControlsEnabled(true);
                document.title = "Connected" + (state.name ? " - " + state.name : "") + " | StealthTech";

                // Update card state
                if (connectionPanel) connectionPanel.dataset.state = "connected";
                if (connectedDevice) connectedDevice.style.display = "";

                // Hide connecting indicators and connection controls for both modes
                if (btConnectingIndicator) btConnectingIndicator.style.display = "none";
                if (btConnectionControls) btConnectionControls.style.display = "none";
                if (srvConnectingIndicator) srvConnectingIndicator.style.display = "none";
                if (srvConnectionControls) srvConnectionControls.style.display = "none";

                // Set device name in the connected panel
                if (connDeviceName) connDeviceName.textContent = state.name || "StealthTech Device";
            } else {
                statusDot.className = "status-dot";
                statusText.textContent = "Disconnected";
                setControlsEnabled(false);
                document.title = "StealthTech Remote";

                // Update card state
                if (connectionPanel) connectionPanel.dataset.state = "disconnected";
                if (connectedDevice) connectedDevice.style.display = "none";

                // Hide connecting indicators, restore connection controls for both modes
                if (btConnectingIndicator) btConnectingIndicator.style.display = "none";
                if (btConnectionControls) btConnectionControls.style.display = "";
                if (srvConnectingIndicator) srvConnectingIndicator.style.display = "none";
                if (srvConnectionControls) srvConnectionControls.style.display = "";

                // Clear firmware text and hide update banner
                if (connFwText) connFwText.textContent = "";
                if (fwBanner) fwBanner.style.display = "none";
            }
        }

        // Sliders (skip updates for any slider currently being dragged)
        var sliderStateKeys = {
            volume: "volume", bass: "bass", treble: "treble",
            "center-volume": "center_volume", "rear-volume": "rear_channel_volume",
            balance: "balance",
        };
        Object.keys(sliderStateKeys).forEach(function (key) {
            var val = state[sliderStateKeys[key]];
            if (val != null && !sliders[key]._dragging) {
                sliders[key].el.value = val;
                sliders[key].val.textContent = toPercent(val, parseInt(sliders[key].el.max, 10));
            }
        });

        // Toggles
        if (state.power != null) {
            toggles.power.el.dataset.active = state.power;
            toggles.power.el.textContent = state.power ? "ON" : "OFF";
            toggles.power.el.setAttribute("aria-pressed", state.power);
        }
        if (state.mute != null) {
            toggles.mute.el.dataset.active = state.mute;
            toggles.mute.el.textContent = state.mute ? "ON" : "OFF";
            toggles.mute.el.setAttribute("aria-pressed", state.mute);
        }
        if (state.quiet_couch != null) {
            toggles.quietCouch.el.dataset.active = state.quiet_couch;
            toggles.quietCouch.el.textContent = state.quiet_couch ? "ON" : "OFF";
            toggles.quietCouch.el.setAttribute("aria-pressed", state.quiet_couch);
        }
        if (state.surround_enabled != null) {
            toggles.surround.el.dataset.active = state.surround_enabled;
            toggles.surround.el.textContent = state.surround_enabled ? "ON" : "OFF";
            toggles.surround.el.setAttribute("aria-pressed", state.surround_enabled);
        }

        // Input buttons (normalize both server and BLE format strings)
        var normalizedInput = state.input ? (inputNormalize[state.input] || state.input) : null;
        $$("[data-input]").forEach(function (btn) {
            btn.classList.toggle("active", normalizedInput === inputMap[btn.dataset.input]);
        });

        // Mode buttons (normalize both server and BLE format strings)
        var normalizedMode = state.sound_mode ? (modeNormalize[state.sound_mode] || state.sound_mode) : null;
        $$(".btn-option[data-mode]").forEach(function (btn) {
            btn.classList.toggle("active", normalizedMode === modeMap[btn.dataset.mode]);
        });

        // Config shape buttons
        if (state.config_shape != null) {
            var normalizedShape = shapeNormalize[state.config_shape] || state.config_shape;
            $$("[data-shape]").forEach(function (btn) {
                btn.classList.toggle("active", normalizedShape === shapeMap[btn.dataset.shape]);
            });
        }

        // Surround (handled by toggle map, but also set aria-pressed)

        // Subwoofer status (read-only)
        if (state.subwoofer_connected != null && subRow && subStatus) {
            subRow.style.display = "";
            if (state.subwoofer_connected) {
                subStatus.textContent = "Connected";
                subStatus.className = "status-badge badge-connected";
            } else {
                subStatus.textContent = "Disconnected";
                subStatus.className = "status-badge badge-disconnected";
            }
        }

        // Media card visibility — show only when input is Bluetooth
        if (mediaCard) {
            var currentInput = state.input ? (inputNormalize[state.input] || state.input) : null;
            mediaCard.style.display = currentInput === "Bluetooth" ? "" : "none";
        }

        // Firmware info
        if (state.firmware) {
            updateFirmwareDisplay(state.firmware);
        }
    }

    // ---------- Firmware display ----------

    function updateFirmwareDisplay(fw) {
        // fw shape: { mcu: {current, latest, up_to_date}, dsp: ..., eq: ..., update_available }
        var components = ["mcu", "dsp", "eq"];
        var labels = { mcu: "MCU", dsp: "DSP", eq: "EQ" };

        // Current version display
        var parts = [];
        components.forEach(function (key) {
            var c = fw[key];
            if (c && c.current) parts.push(labels[key] + " " + c.current);
        });

        if (parts.length > 0) {
            var fwText = $("#connected-firmware-text");
            if (fwText) fwText.textContent = parts.join(" / ");
        }

        // Update banner with upgrade details
        var banner = $("#firmware-update-banner");
        if (banner) {
            if (fw.update_available) {
                var upgrades = [];
                components.forEach(function (key) {
                    var c = fw[key];
                    if (c && c.current && c.latest && c.up_to_date === false) {
                        upgrades.push(labels[key] + " " + c.current + " \u2192 " + c.latest);
                    }
                });
                var msg = "Firmware update available";
                if (upgrades.length > 0) msg += ": " + upgrades.join(", ");
                msg += ". ";
                banner.innerHTML = msg +
                    '<a href="https://www.lovesac.com/stealthtech-firmware-updates" target="_blank" rel="noopener">Learn more</a>';
                banner.style.display = "";
            } else {
                banner.style.display = "none";
            }
        }
    }

    // ---------- Slider handlers ----------

    Object.keys(sliders).forEach(function (key) {
        var slider = sliders[key];
        slider._dragging = false;

        // Track drag state so updateUI won't overwrite mid-drag
        slider.el.addEventListener("pointerdown", function () {
            slider._dragging = true;
            slider._committed = parseInt(slider.el.value, 10);
        });

        // Immediate visual feedback (percentage)
        slider.el.addEventListener("input", function () {
            slider.val.textContent = toPercent(parseInt(slider.el.value, 10), parseInt(slider.el.max, 10));
        });

        // Send only when the user releases the slider
        slider.el.addEventListener("change", function () {
            slider._dragging = false;
            var value = parseInt(slider.el.value, 10);
            var prev = slider._committed;
            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send(key, value).then(function (state) {
                slider._committed = value;
                if (state) updateUI(state);
            }).catch(function (e) {
                if (prev != null) {
                    slider.el.value = prev;
                    slider.val.textContent = toPercent(prev, parseInt(slider.el.max, 10));
                }
                showError("Failed to set " + key + ": " + e.message);
            });
        });
    });

    // ---------- Toggle handlers ----------

    Object.keys(toggles).forEach(function (key) {
        var toggle = toggles[key];
        toggle.el.addEventListener("click", function () {
            var currentValue = toggle.el.dataset.active === "true";
            var newValue = !currentValue;

            // Optimistic update
            toggle.el.dataset.active = newValue;
            toggle.el.textContent = newValue ? "ON" : "OFF";
            toggle.el.setAttribute("aria-pressed", newValue);

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send(key, newValue).then(function (state) {
                if (state) updateUI(state);
            }).catch(function (e) {
                toggle.el.dataset.active = currentValue;
                toggle.el.textContent = currentValue ? "ON" : "OFF";
                toggle.el.setAttribute("aria-pressed", currentValue);
                showError("Failed to toggle " + key + ": " + e.message);
            });
        });
    });

    // ---------- Input button handlers ----------

    $$("[data-input]").forEach(function (btn) {
        btn.addEventListener("click", function () {
            var input = btn.dataset.input;
            var prevActive = $("[data-input].active");

            // Optimistic update
            $$("[data-input]").forEach(function (b) { b.classList.remove("active"); });
            btn.classList.add("active");

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send("input", input).then(function (state) {
                if (state) updateUI(state);
            }).catch(function (e) {
                $$("[data-input]").forEach(function (b) { b.classList.remove("active"); });
                if (prevActive) prevActive.classList.add("active");
                showError("Failed to set input: " + e.message);
            });
        });
    });

    // ---------- Mode button handlers ----------

    $$(".btn-option[data-mode]").forEach(function (btn) {
        btn.addEventListener("click", function () {
            var mode = btn.dataset.mode;
            var prevActive = $(".btn-option[data-mode].active");

            // Optimistic update
            $$(".btn-option[data-mode]").forEach(function (b) { b.classList.remove("active"); });
            btn.classList.add("active");

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send("mode", mode).then(function (state) {
                if (state) updateUI(state);
            }).catch(function (e) {
                $$(".btn-option[data-mode]").forEach(function (b) { b.classList.remove("active"); });
                if (prevActive) prevActive.classList.add("active");
                showError("Failed to set mode: " + e.message);
            });
        });
    });

    // ---------- Shape button handlers ----------

    $$("[data-shape]").forEach(function (btn) {
        btn.addEventListener("click", function () {
            var shape = btn.dataset.shape;
            var prevActive = $("[data-shape].active");

            // Optimistic update
            $$("[data-shape]").forEach(function (b) { b.classList.remove("active"); });
            btn.classList.add("active");

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send("config-shape", shape).then(function (state) {
                if (state) updateUI(state);
            }).catch(function (e) {
                $$("[data-shape]").forEach(function (b) { b.classList.remove("active"); });
                if (prevActive) prevActive.classList.add("active");
                showError("Failed to set shape: " + e.message);
            });
        });
    });

    // ---------- Media control handlers ----------

    var mediaButtons = [
        { id: "play-pause-btn", action: "play-pause", value: 1, label: "Play/Pause" },
        { id: "skip-fwd-btn",   action: "skip",       value: 0, label: "Skip forward" },
        { id: "skip-back-btn",  action: "skip",       value: 1, label: "Skip back" },
    ];

    mediaButtons.forEach(function (cfg) {
        var btn = $("#" + cfg.id);
        if (!btn) return;
        btn.addEventListener("click", function () {
            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send(cfg.action, cfg.value).catch(function (e) {
                showError(cfg.label + " failed: " + e.message);
            });
        });
    });

    // ---------- Unified disconnect button ----------

    var deviceDisconnectBtn = $("#device-disconnect-btn");
    if (deviceDisconnectBtn) {
        deviceDisconnectBtn.addEventListener("click", function () {
            var t = getActiveTransport();
            if (t && t.disconnect) t.disconnect();
        });
    }

    // ---------- Public API ----------

    window.StealthTech = {
        registerTransport: registerTransport,
        getActiveTransport: getActiveTransport,
        activeMode: function () { return activeMode; },
        updateUI: updateUI,
        addLogEntry: addLogEntry,
        showError: showError,
        $: $,
        $$: $$,
    };
})();
