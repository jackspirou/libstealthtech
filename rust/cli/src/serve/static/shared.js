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

    // Profile elements
    var profileButtonsContainer = $("#profile-buttons");
    var newProfileBtn = $("#new-profile-btn");

    // ---------- Percentage display helper ----------

    function toPercent(value, max) {
        if (!max) return "0%";
        return Math.round((value / max) * 100) + "%";
    }

    function setSlider(key, value) {
        if (sliders[key]._dragging || sliders[key]._pending) return;
        sliders[key].el.value = value;
        sliders[key].val.textContent = toPercent(value, parseInt(sliders[key].el.max, 10));
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

        if (themeIcon) {
            themeIcon.textContent = theme === "auto" ? "\u2699" : theme === "light" ? "\u2600" : "\u263E";
        }
        localStorage.setItem(THEME_KEY, theme);
    }

    if (themeToggle) {
        themeToggle.addEventListener("click", function () {
            currentThemeIndex = (currentThemeIndex + 1) % themes.length;
            applyTheme();
        });
    }

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
        logEmpty = logContainer.querySelector("#log-empty");
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

    function setControlsEnabled(enabled, opts) {
        var skipPower = opts && opts.skipPower;
        Object.keys(sliders).forEach(function (key) {
            sliders[key].el.disabled = !enabled;
        });
        Object.keys(toggles).forEach(function (key) {
            if (skipPower && key === "power") return;
            toggles[key].el.disabled = !enabled;
        });
        $$("[data-input]").forEach(function (btn) { btn.disabled = !enabled; });
        $$(".btn-option[data-mode]").forEach(function (btn) { btn.disabled = !enabled; });
        $$(".profile-btn-wrap .btn-option").forEach(function (btn) { btn.disabled = !enabled; });
        if (newProfileBtn) newProfileBtn.disabled = !enabled;
        $$("[data-shape]").forEach(function (btn) { btn.disabled = !enabled; });
        var mediaBtn = $("#play-pause-btn");
        if (mediaBtn) mediaBtn.disabled = !enabled;
        var skipFwd = $("#skip-fwd-btn");
        if (skipFwd) skipFwd.disabled = !enabled;
        var skipBack = $("#skip-back-btn");
        if (skipBack) skipBack.disabled = !enabled;
    }

    // Start with controls disabled
    setControlsEnabled(false);

    // ---------- Standby mode (power off) ----------

    // Authoritative power state flag, distinct from the DOM attribute
    // which may not be set before the first Power notification arrives.
    var devicePoweredOn = false;

    function setStandbyMode(standby) {
        if (devicePoweredOn === !standby) return;
        devicePoweredOn = !standby;
        setControlsEnabled(!standby, { skipPower: true });
    }

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
                devicePoweredOn = false;
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
            if (val != null) setSlider(key, val);
        });

        // Toggles
        if (state.power != null) {
            toggles.power.el.classList.toggle("active", !!state.power);
            toggles.power.el.setAttribute("aria-pressed", state.power);
            setStandbyMode(!state.power);
        }
        if (state.mute != null) {
            toggles.mute.el.classList.toggle("active", !!state.mute);
            toggles.mute.el.setAttribute("aria-pressed", state.mute);
        }
        if (state.quiet_couch != null) {
            toggles.quietCouch.el.classList.toggle("active", !!state.quiet_couch);
            toggles.quietCouch.el.setAttribute("aria-pressed", state.quiet_couch);
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

        updateSaveRowVisibility();

        // Config shape buttons
        if (state.config_shape != null) {
            var normalizedShape = shapeNormalize[state.config_shape] || state.config_shape;
            $$("[data-shape]").forEach(function (btn) {
                btn.classList.toggle("active", normalizedShape === shapeMap[btn.dataset.shape]);
            });
        }

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

        if (parts.length > 0 && connFwText) {
            connFwText.textContent = parts.join(" / ");
        }

        // Update banner with upgrade details
        if (fwBanner) {
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
                fwBanner.innerHTML = msg +
                    '<a href="https://www.lovesac.com/stealthtech-firmware-updates" target="_blank" rel="noopener">Learn more</a>';
                fwBanner.style.display = "";
            } else {
                fwBanner.style.display = "none";
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
            if (!devicePoweredOn) {
                if (slider._committed != null) {
                    slider.el.value = slider._committed;
                    slider.val.textContent = toPercent(slider._committed, parseInt(slider.el.max, 10));
                }
                return;
            }
            var value = parseInt(slider.el.value, 10);
            var prev = slider._committed;
            var t = getActiveTransport();
            if (!t || !t.send) return;

            slider._pending = true;
            t.send(key, value).then(function (state) {
                slider._committed = value;
                slider._pending = false;
                if (state) updateUI(state);
                autoUpdateActiveProfile();
                trackSuccess();
            }).catch(function (e) {
                slider._pending = false;
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
            if (key !== "power" && !devicePoweredOn) return;
            var currentValue = toggle.el.classList.contains("active");
            var newValue = !currentValue;

            // Optimistic update
            toggle.el.classList.toggle("active", newValue);
            toggle.el.setAttribute("aria-pressed", newValue);

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send(key, newValue).then(function (state) {
                if (state) updateUI(state);
                trackSuccess();
            }).catch(function (e) {
                toggle.el.classList.toggle("active", currentValue);
                toggle.el.setAttribute("aria-pressed", currentValue);
                showError("Failed to toggle " + key + ": " + e.message);
            });
        });
    });

    // ---------- Input button handlers ----------

    $$("[data-input]").forEach(function (btn) {
        btn.addEventListener("click", function () {
            if (!devicePoweredOn) return;
            var input = btn.dataset.input;
            var prevActive = $("[data-input].active");

            // Optimistic update
            $$("[data-input]").forEach(function (b) { b.classList.remove("active"); });
            btn.classList.add("active");

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send("input", input).then(function (state) {
                if (state) updateUI(state);
                autoUpdateActiveProfile();
                trackSuccess();
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
            if (!devicePoweredOn) return;
            var wasActive = btn.classList.contains("active");
            var mode = wasActive ? "manual" : btn.dataset.mode;
            var prevActive = $(".btn-option[data-mode].active");

            // Optimistic update
            $$(".btn-option[data-mode]").forEach(function (b) { b.classList.remove("active"); });
            if (!wasActive) btn.classList.add("active");

            updateSaveRowVisibility();

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send("mode", mode).then(function (state) {
                if (state) updateUI(state);
                autoUpdateActiveProfile();
                trackSuccess();
            }).catch(function (e) {
                $$(".btn-option[data-mode]").forEach(function (b) { b.classList.remove("active"); });
                if (prevActive) prevActive.classList.add("active");
                updateSaveRowVisibility();
                showError("Failed to set mode: " + e.message);
            });
        });
    });

    // ---------- Shape button handlers ----------

    $$("[data-shape]").forEach(function (btn) {
        btn.addEventListener("click", function () {
            if (!devicePoweredOn) return;
            var shape = btn.dataset.shape;
            var prevActive = $("[data-shape].active");

            // Optimistic update
            $$("[data-shape]").forEach(function (b) { b.classList.remove("active"); });
            btn.classList.add("active");

            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send("config-shape", shape).then(function (state) {
                if (state) updateUI(state);
                trackSuccess();
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
            if (!devicePoweredOn) return;
            var t = getActiveTransport();
            if (!t || !t.send) return;

            t.send(cfg.action, cfg.value).then(function () {
                trackSuccess();
            }).catch(function (e) {
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

    // ---------- Tip tooltip ----------

    var TIP_ACTIONS_KEY = "stealthtech-tip-actions";
    var TIP_DISMISSED_KEY = "stealthtech-tip-dismissed-count";
    var TIP_CLICKED_KEY = "stealthtech-tip-clicked";
    var FIRST_SHOW = 5;
    var RESHOW_GAP = 25;

    var tipTooltip = $("#tip-tooltip");
    var tipClose = $("#tip-tooltip-close");
    var kofiLink = $("#kofi-link");

    function getTipCount() {
        return parseInt(localStorage.getItem(TIP_ACTIONS_KEY) || "0", 10);
    }

    function trackSuccess() {
        if (localStorage.getItem(TIP_CLICKED_KEY)) return;
        var count = getTipCount() + 1;
        localStorage.setItem(TIP_ACTIONS_KEY, count);

        var dismissed = parseInt(localStorage.getItem(TIP_DISMISSED_KEY) || "0", 10);
        if (dismissed === 0 && count >= FIRST_SHOW) {
            showTip();
        } else if (dismissed > 0 && count >= dismissed + RESHOW_GAP) {
            showTip();
        }
    }

    function showTip() {
        if (tipTooltip) tipTooltip.classList.add("visible");
    }

    function hideTip() {
        if (tipTooltip) tipTooltip.classList.remove("visible");
    }

    if (tipClose) {
        tipClose.addEventListener("click", function (e) {
            e.preventDefault();
            localStorage.setItem(TIP_DISMISSED_KEY, getTipCount());
            hideTip();
        });
    }

    if (kofiLink) {
        kofiLink.addEventListener("click", function () {
            localStorage.setItem(TIP_CLICKED_KEY, "1");
            hideTip();
        });
    }

    // ---------- Custom Profiles ----------

    var PROFILES_KEY = "stealthtech-profiles";
    var activeProfileName = null;
    var applyingProfile = false;

    function updateSaveRowVisibility() {
        if (!newProfileBtn) return;
        newProfileBtn.style.display = activeProfileName ? "none" : "";
    }

    function autoUpdateActiveProfile() {
        if (!activeProfileName || applyingProfile) return;
        var profiles = loadProfiles();
        for (var i = 0; i < profiles.length; i++) {
            if (profiles[i].name === activeProfileName) {
                var eq = buildProfileFromSliders();
                eq.name = activeProfileName;
                profiles[i] = eq;
                saveProfilesToStorage(profiles);
                break;
            }
        }
    }

    function loadProfiles() {
        try {
            return JSON.parse(localStorage.getItem(PROFILES_KEY)) || [];
        } catch (e) {
            return [];
        }
    }

    function saveProfilesToStorage(profiles) {
        localStorage.setItem(PROFILES_KEY, JSON.stringify(profiles));
    }

    var _popoverOutsideClick = null;

    function closeProfilePopover() {
        var existing = $(".profile-popover");
        if (existing) existing.remove();
        if (_popoverOutsideClick) {
            document.removeEventListener("click", _popoverOutsideClick);
            _popoverOutsideClick = null;
        }
    }

    function showProfilePopover(menuBtn, profileName) {
        closeProfilePopover();
        var pop = document.createElement("div");
        pop.className = "profile-popover";

        var editBtn = document.createElement("button");
        editBtn.className = "profile-popover-action";
        editBtn.textContent = "Edit Name";
        editBtn.addEventListener("click", function () {
            closeProfilePopover();
            renameProfile(profileName);
        });

        var delBtn = document.createElement("button");
        delBtn.className = "profile-popover-action danger";
        delBtn.textContent = "Delete";
        delBtn.addEventListener("click", function () {
            closeProfilePopover();
            deleteProfile(profileName);
        });

        pop.appendChild(editBtn);
        pop.appendChild(delBtn);
        menuBtn.parentNode.appendChild(pop);

        function onOutsideClick(e) {
            if (!pop.contains(e.target) && e.target !== menuBtn) {
                closeProfilePopover();
            }
        }
        _popoverOutsideClick = onOutsideClick;
        setTimeout(function () {
            document.addEventListener("click", onOutsideClick);
        }, 0);
    }

    function renameProfile(oldName) {
        var m = showModal(
            '<h3>Rename Profile</h3>' +
            '<input type="text" class="profile-name-input" id="rename-profile-input" ' +
                'value="' + escapeHtml(oldName) + '" maxlength="20" aria-label="Profile name">' +
            '<div class="confirm-actions" style="margin-top:16px">' +
                '<button class="btn btn-secondary btn-sm confirm-no">Cancel</button>' +
                '<button class="btn btn-primary btn-sm confirm-yes">Save</button>' +
            '</div>'
        );
        if (!m) return;
        var input = m.overlay.querySelector("#rename-profile-input");
        input.focus();
        input.select();
        m.overlay.querySelector(".confirm-no").addEventListener("click", m.close);
        m.overlay.querySelector(".confirm-yes").addEventListener("click", function () {
            var newName = input.value.trim();
            if (!newName) { showError("Enter a profile name"); return; }
            if (newName === oldName) { m.close(); return; }
            var profiles = loadProfiles();
            for (var i = 0; i < profiles.length; i++) {
                if (profiles[i].name === newName) {
                    showError("A profile named \"" + newName + "\" already exists");
                    return;
                }
            }
            for (var i = 0; i < profiles.length; i++) {
                if (profiles[i].name === oldName) {
                    profiles[i].name = newName;
                    break;
                }
            }
            saveProfilesToStorage(profiles);
            if (activeProfileName === oldName) activeProfileName = newName;
            renderProfiles();
            m.close();
        });
    }

    function renderProfiles() {
        if (!profileButtonsContainer) return;
        var profiles = loadProfiles();
        profileButtonsContainer.innerHTML = "";

        profiles.forEach(function (p) {
            var wrap = document.createElement("div");
            wrap.className = "profile-btn-wrap";

            var btn = document.createElement("button");
            btn.className = "btn btn-option";
            btn.textContent = p.name;
            btn.disabled = !devicePoweredOn;
            btn.addEventListener("click", function () {
                if (activeProfileName === p.name) {
                    activeProfileName = null;
                    renderProfiles();
                    return;
                }
                applyProfile(p);
            });

            if (activeProfileName === p.name) {
                btn.classList.add("active");
            }

            var menuBtn = document.createElement("button");
            menuBtn.className = "profile-menu-btn";
            menuBtn.textContent = "\u22ee";
            menuBtn.title = "Profile options";
            menuBtn.addEventListener("click", function (e) {
                e.stopPropagation();
                showProfilePopover(menuBtn, p.name);
            });

            wrap.appendChild(btn);
            wrap.appendChild(menuBtn);
            profileButtonsContainer.appendChild(wrap);
        });

        updateSaveRowVisibility();
    }

    function buildProfileFromSliders() {
        var activeModeBtn = $(".btn-option[data-mode].active");
        var activeInputBtn = $("[data-input].active");
        return {
            volume: parseInt(sliders.volume.el.value, 10),
            soundMode: activeModeBtn ? activeModeBtn.dataset.mode : "manual",
            input: activeInputBtn ? activeInputBtn.dataset.input : null,
            bass: parseInt(sliders.bass.el.value, 10),
            treble: parseInt(sliders.treble.el.value, 10),
            balance: parseInt(sliders.balance.el.value, 10),
            centerVolume: parseInt(sliders["center-volume"].el.value, 10),
            rearVolume: parseInt(sliders["rear-volume"].el.value, 10),
        };
    }

    function applyProfile(profile) {
        if (!devicePoweredOn) return;
        var t = getActiveTransport();
        if (!t || !t.send) return;

        applyingProfile = true;
        activeProfileName = profile.name;
        renderProfiles();

        // Build command chain: input, mode, volume, then EQ values
        var chain = Promise.resolve();

        if (profile.input) {
            chain = chain.then(function () { return t.send("input", profile.input); });
        }

        var mode = profile.soundMode || "manual";
        chain = chain.then(function () { return t.send("mode", mode); });

        if (profile.volume != null) {
            chain = chain.then(function () { return t.send("volume", profile.volume); });
        }

        chain.then(function () {
            return t.send("bass", profile.bass);
        }).then(function () {
            return t.send("treble", profile.treble);
        }).then(function () {
            return t.send("balance", profile.balance);
        }).then(function () {
            return t.send("center-volume", profile.centerVolume);
        }).then(function () {
            return t.send("rear-volume", profile.rearVolume);
        }).then(function () {
            // Update UI to reflect profile values
            if (profile.volume != null) setSlider("volume", profile.volume);
            setSlider("bass", profile.bass);
            setSlider("treble", profile.treble);
            setSlider("balance", profile.balance);
            setSlider("center-volume", profile.centerVolume);
            setSlider("rear-volume", profile.rearVolume);

            // Update mode buttons
            $$(".btn-option[data-mode]").forEach(function (b) {
                b.classList.toggle("active", b.dataset.mode === mode);
            });

            // Update input buttons
            if (profile.input) {
                $$("[data-input]").forEach(function (b) {
                    b.classList.toggle("active", b.dataset.input === profile.input);
                });
            }

            applyingProfile = false;
            trackSuccess();
        }).catch(function (e) {
            applyingProfile = false;
            activeProfileName = null;
            renderProfiles();
            showError("Failed to apply profile: " + e.message);
        });
    }

    function deleteProfile(name) {
        showConfirm("Delete Profile", 'Delete "' + name + '"? This cannot be undone.', function () {
            var profiles = loadProfiles().filter(function (p) { return p.name !== name; });
            saveProfilesToStorage(profiles);
            if (activeProfileName === name) activeProfileName = null;
            renderProfiles();
        });
    }

    function showModal(innerHtml) {
        if ($(".confirm-overlay")) return null;
        var overlay = document.createElement("div");
        overlay.className = "confirm-overlay";
        overlay.innerHTML =
            '<div class="confirm-dialog">' +
                '<button class="confirm-close" aria-label="Close">&times;</button>' +
                innerHtml +
            '</div>';

        function close() {
            document.removeEventListener("keydown", onKey);
            overlay.classList.remove("visible");
            setTimeout(function () { overlay.remove(); }, 200);
        }

        function onKey(e) {
            if (e.key === "Escape") close();
        }

        overlay.querySelector(".confirm-close").addEventListener("click", close);
        overlay.addEventListener("click", function (e) {
            if (e.target === overlay) close();
        });
        document.addEventListener("keydown", onKey);

        document.body.appendChild(overlay);
        overlay.offsetHeight; // eslint-disable-line no-unused-expressions
        overlay.classList.add("visible");

        return { overlay: overlay, close: close };
    }

    function showConfirm(title, message, onConfirm) {
        var m = showModal(
            '<h3>' + escapeHtml(title) + '</h3>' +
            '<p>' + escapeHtml(message) + '</p>' +
            '<div class="confirm-actions">' +
                '<button class="btn btn-secondary btn-sm confirm-no">No</button>' +
                '<button class="btn btn-danger btn-sm confirm-yes">Yes</button>' +
            '</div>'
        );
        if (!m) return;
        m.overlay.querySelector(".confirm-no").addEventListener("click", m.close);
        m.overlay.querySelector(".confirm-yes").addEventListener("click", function () {
            m.close();
            onConfirm();
        });
    }

    if (newProfileBtn) {
        newProfileBtn.addEventListener("click", function () {
            var m = showModal(
                '<h3>New Profile</h3>' +
                '<input type="text" class="profile-name-input" id="new-profile-input" ' +
                    'placeholder="Profile name" maxlength="20" aria-label="Profile name">' +
                '<div class="confirm-actions" style="margin-top:16px">' +
                    '<button class="btn btn-secondary btn-sm confirm-no">Cancel</button>' +
                    '<button class="btn btn-primary btn-sm confirm-yes">Save</button>' +
                '</div>'
            );
            if (!m) return;
            var input = m.overlay.querySelector("#new-profile-input");
            input.focus();
            m.overlay.querySelector(".confirm-no").addEventListener("click", m.close);
            m.overlay.querySelector(".confirm-yes").addEventListener("click", function () {
                var name = input.value.trim();
                if (!name) { showError("Enter a profile name"); return; }
                var profiles = loadProfiles();
                var eq = buildProfileFromSliders();
                eq.name = name;
                var idx = -1;
                for (var i = 0; i < profiles.length; i++) {
                    if (profiles[i].name === name) { idx = i; break; }
                }
                if (idx >= 0) {
                    profiles[idx] = eq;
                } else {
                    profiles.push(eq);
                }
                saveProfilesToStorage(profiles);
                activeProfileName = name;
                renderProfiles();
                m.close();
            });
        });
    }

    renderProfiles();

    // ---------- Card Collapse ----------

    var COLLAPSED_KEY = "stealthtech-card-collapsed";

    function loadCollapsed() {
        try { return JSON.parse(localStorage.getItem(COLLAPSED_KEY)) || {}; } catch (e) { return {}; }
    }

    function saveCollapsed(obj) {
        localStorage.setItem(COLLAPSED_KEY, JSON.stringify(obj));
    }

    function setCardCollapsed(card, collapsed, animate) {
        var body = card.querySelector(".card-body");
        var header = card.querySelector(".card-header");
        if (!body || !header) return;

        if (collapsed) {
            if (animate) {
                body.style.maxHeight = body.scrollHeight + "px";
                // Force reflow so the browser registers the start value
                body.offsetHeight; // eslint-disable-line no-unused-expressions
                body.style.maxHeight = "0";
            }
            card.classList.add("collapsed");
            header.setAttribute("aria-expanded", "false");
        } else {
            card.classList.remove("collapsed");
            header.setAttribute("aria-expanded", "true");
            if (animate) {
                body.style.maxHeight = body.scrollHeight + "px";
                var onEnd = function () {
                    body.removeEventListener("transitionend", onEnd);
                    if (!card.classList.contains("collapsed")) {
                        body.style.maxHeight = "";
                    }
                };
                body.addEventListener("transitionend", onEnd);
            } else {
                body.style.maxHeight = "";
            }
        }

        var id = card.dataset.cardId;
        if (id) {
            var state = loadCollapsed();
            state[id] = collapsed;
            saveCollapsed(state);
        }
    }

    function initCollapse() {
        var state = loadCollapsed();

        // Default log card to collapsed if user hasn't explicitly toggled it
        if (!state.hasOwnProperty("log")) {
            state["log"] = true;
        }

        $$("[data-card-id]").forEach(function (card) {
            var id = card.dataset.cardId;
            if (state[id]) {
                setCardCollapsed(card, true, false);
            }
        });

        $("main").addEventListener("click", function (e) {
            var header = e.target.closest(".card-header");
            if (!header) return;
            if (e.target.closest(".drag-handle")) return;
            if (e.target.closest(".log-clear-btn")) return;
            var card = header.closest("[data-card-id]");
            if (!card) return;
            setCardCollapsed(card, !card.classList.contains("collapsed"), true);
        });

        $("main").addEventListener("keydown", function (e) {
            if (e.key !== "Enter" && e.key !== " ") return;
            var header = e.target.closest(".card-header");
            if (!header || e.target.closest(".drag-handle")) return;
            e.preventDefault();
            var card = header.closest("[data-card-id]");
            if (!card) return;
            setCardCollapsed(card, !card.classList.contains("collapsed"), true);
        });
    }

    // ---------- Card Drag-and-Drop ----------

    var ORDER_KEY = "stealthtech-card-order";
    var DEFAULT_ORDER = ["connection", "system", "input", "media", "mode", "volume", "eq", "shape", "log"];

    function loadOrder() {
        try { return JSON.parse(localStorage.getItem(ORDER_KEY)); } catch (e) { return null; }
    }

    function saveOrder(order) {
        localStorage.setItem(ORDER_KEY, JSON.stringify(order));
    }

    function applyOrder(order) {
        var main = $("main");
        if (!main) return;

        // Build lookup of existing cards
        var cards = {};
        $$("[data-card-id]").forEach(function (card) {
            cards[card.dataset.cardId] = card;
        });

        // Ensure connection is always first
        var sorted = ["connection"];
        order.forEach(function (id) {
            if (id !== "connection" && cards[id] && sorted.indexOf(id) === -1) {
                sorted.push(id);
            }
        });
        // Append any cards not in the saved order
        DEFAULT_ORDER.forEach(function (id) {
            if (sorted.indexOf(id) === -1 && cards[id]) {
                sorted.push(id);
            }
        });

        sorted.forEach(function (id) {
            if (cards[id]) main.appendChild(cards[id]);
        });
    }

    function getCurrentOrder() {
        var order = [];
        $$("[data-card-id]").forEach(function (card) {
            order.push(card.dataset.cardId);
        });
        return order;
    }

    function initDragAndDrop() {
        var main = $("main");
        if (!main) return;

        var dropIndicator = document.createElement("div");
        dropIndicator.className = "drop-indicator";
        document.body.appendChild(dropIndicator);

        var dragging = null;
        var placeholder = null;
        var startX = 0;
        var startY = 0;
        var offsetX = 0;
        var offsetY = 0;
        var dragStarted = false;
        var DEAD_ZONE = 5;

        main.addEventListener("pointerdown", function (e) {
            var handle = e.target.closest(".drag-handle");
            if (!handle) return;
            var card = handle.closest("[data-card-id]");
            if (!card || card.dataset.pinned === "true") return;

            e.preventDefault();
            handle.setPointerCapture(e.pointerId);

            dragging = card;
            startX = e.clientX;
            startY = e.clientY;
            dragStarted = false;

            var rect = card.getBoundingClientRect();
            offsetX = e.clientX - rect.left;
            offsetY = e.clientY - rect.top;

            var onMove = function (ev) {
                if (!dragging) return;

                if (!dragStarted) {
                    var dx = ev.clientX - startX;
                    var dy = ev.clientY - startY;
                    if (Math.sqrt(dx * dx + dy * dy) < DEAD_ZONE) return;
                    dragStarted = true;

                    // Create placeholder
                    var r = dragging.getBoundingClientRect();
                    placeholder = document.createElement("div");
                    placeholder.className = "card-placeholder";
                    placeholder.style.height = r.height + "px";
                    dragging.parentNode.insertBefore(placeholder, dragging);

                    // Make card fixed
                    dragging.classList.add("dragging");
                    dragging.style.width = r.width + "px";
                    dragging.style.height = r.height + "px";
                }

                dragging.style.left = (ev.clientX - offsetX) + "px";
                dragging.style.top = (ev.clientY - offsetY) + "px";

                // Calculate drop position
                var cards = [];
                $$("[data-card-id]").forEach(function (c) {
                    if (c === dragging || c.dataset.pinned === "true") return;
                    if (c.style.display === "none") return;
                    var cr = c.getBoundingClientRect();
                    cards.push({ el: c, mid: cr.top + cr.height / 2 });
                });

                // Also include placeholder in positioning
                var insertBefore = null;
                for (var i = 0; i < cards.length; i++) {
                    if (ev.clientY < cards[i].mid) {
                        insertBefore = cards[i].el;
                        break;
                    }
                }

                // Show drop indicator
                if (insertBefore) {
                    var ir = insertBefore.getBoundingClientRect();
                    var mainR = main.getBoundingClientRect();
                    dropIndicator.style.display = "block";
                    dropIndicator.style.top = (ir.top - 2) + "px";
                    dropIndicator.style.left = mainR.left + "px";
                    dropIndicator.style.width = mainR.width + "px";
                } else if (cards.length > 0) {
                    var last = cards[cards.length - 1].el.getBoundingClientRect();
                    var mainR2 = main.getBoundingClientRect();
                    dropIndicator.style.display = "block";
                    dropIndicator.style.top = (last.bottom + 2) + "px";
                    dropIndicator.style.left = mainR2.left + "px";
                    dropIndicator.style.width = mainR2.width + "px";
                }
            };

            var onUp = function () {
                if (!dragging) return;

                handle.removeEventListener("pointermove", onMove);
                handle.removeEventListener("pointerup", onUp);
                handle.removeEventListener("pointercancel", onUp);

                if (dragStarted) {
                    // Find drop target
                    var cards = [];
                    $$("[data-card-id]").forEach(function (c) {
                        if (c === dragging || c.dataset.pinned === "true") return;
                        if (c.style.display === "none" && c !== dragging) return;
                        var cr = c.getBoundingClientRect();
                        cards.push({ el: c, mid: cr.top + cr.height / 2 });
                    });

                    var insertBefore = null;
                    var lastPointerY = parseInt(dragging.style.top, 10) + offsetY;
                    for (var i = 0; i < cards.length; i++) {
                        if (lastPointerY < cards[i].mid) {
                            insertBefore = cards[i].el;
                            break;
                        }
                    }

                    // Remove placeholder
                    if (placeholder && placeholder.parentNode) {
                        placeholder.parentNode.removeChild(placeholder);
                    }

                    // Reset card styles
                    dragging.classList.remove("dragging");
                    dragging.style.width = "";
                    dragging.style.height = "";
                    dragging.style.left = "";
                    dragging.style.top = "";
                    dragging.style.position = "";

                    // Insert at new position
                    if (insertBefore) {
                        main.insertBefore(dragging, insertBefore);
                    } else {
                        main.appendChild(dragging);
                    }

                    dropIndicator.style.display = "none";
                    saveOrder(getCurrentOrder());
                }

                placeholder = null;
                dragging = null;
                dragStarted = false;
            };

            handle.addEventListener("pointermove", onMove);
            handle.addEventListener("pointerup", onUp);
            handle.addEventListener("pointercancel", onUp);
        });
    }

    // ---------- Card Layout Init ----------

    function initCardLayout() {
        var savedOrder = loadOrder();
        if (savedOrder) applyOrder(savedOrder);
        initCollapse();
        initDragAndDrop();
    }

    initCardLayout();

    // ---------- Reset Layout ----------

    function resetLayout() {
        localStorage.removeItem(COLLAPSED_KEY);
        localStorage.removeItem(ORDER_KEY);
        applyOrder(DEFAULT_ORDER);
        $$("[data-card-id]").forEach(function (card) {
            var collapse = card.dataset.cardId === "log";
            setCardCollapsed(card, collapse, false);
        });
    }

    var resetBtn = $("#reset-layout-btn");
    if (resetBtn) {
        resetBtn.addEventListener("click", resetLayout);
    }

    // ---------- Settings Modal ----------

    function showSettings() {
        var theme = themes[currentThemeIndex];
        var m = showModal(
            '<h3>Settings</h3>' +
            '<div class="settings-section">' +
                '<span class="settings-label">Appearance</span>' +
                '<div class="button-group">' +
                    '<button class="btn btn-option settings-theme-btn' + (theme === "auto" ? " active" : "") + '" data-theme-val="auto">Auto</button>' +
                    '<button class="btn btn-option settings-theme-btn' + (theme === "light" ? " active" : "") + '" data-theme-val="light">Light</button>' +
                    '<button class="btn btn-option settings-theme-btn' + (theme === "dark" ? " active" : "") + '" data-theme-val="dark">Dark</button>' +
                '</div>' +
            '</div>' +
            '<div class="settings-section">' +
                '<span class="settings-label">Layout</span>' +
                '<button class="btn btn-secondary settings-reset-btn">Reset Layout</button>' +
            '</div>' +
            '<div class="settings-section">' +
                '<span class="settings-label">Tip Memory</span>' +
                '<button class="btn btn-secondary settings-reset-tip-btn">Reset Tip</button>' +
            '</div>'
        );
        if (!m) return;

        var themeBtns = m.overlay.querySelectorAll(".settings-theme-btn");
        themeBtns.forEach(function (btn) {
            btn.addEventListener("click", function () {
                var val = btn.getAttribute("data-theme-val");
                currentThemeIndex = themes.indexOf(val);
                applyTheme();
                themeBtns.forEach(function (b) { b.classList.remove("active"); });
                btn.classList.add("active");
            });
        });

        m.overlay.querySelector(".settings-reset-btn").addEventListener("click", function () {
            resetLayout();
            m.close();
        });

        m.overlay.querySelector(".settings-reset-tip-btn").addEventListener("click", function () {
            StealthTech.resetTip();
            m.close();
        });
    }

    var settingsBtn = $("#settings-btn");
    if (settingsBtn) {
        settingsBtn.addEventListener("click", showSettings);
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
        resetTip: function () {
            localStorage.removeItem(TIP_ACTIONS_KEY);
            localStorage.removeItem(TIP_DISMISSED_KEY);
            localStorage.removeItem(TIP_CLICKED_KEY);
            hideTip();
            console.log("Tip counter reset");
        },
        resetLayout: resetLayout,
    };
})();
