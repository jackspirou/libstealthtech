# Google Analytics — StealthTech

**Property ID:** `G-S8RFN5QZJH`
**Tag location:** `rust/cli/src/serve/static/index.html` (immediately after `<head>`)
**Tracking code:** `rust/cli/src/serve/static/shared.js` — all events use a `trackEvent(name, params)` wrapper around `gtag("event", ...)`

---

## Events

All custom events tracked across the app.

### Navigation & UI

| Event | Parameters | Trigger |
|---|---|---|
| `docs_click` | — | Docs nav link clicked |
| `github_click` | — | GitHub nav link clicked |
| `settings_open` | — | Settings gear button clicked |
| `theme_change` | `theme` | Theme cycled (auto/light/dark) |
| `layout_reset` | — | Card layout reset to defaults |
| `card_toggle` | `card`, `collapsed` | Card section expanded or collapsed |
| `log_clear` | — | Live notifications log cleared |

### Connection

| Event | Parameters | Trigger |
|---|---|---|
| `transport_switch` | `mode` | User switches between Server BLE / Web Bluetooth tabs |
| `device_connected` | `device_name`, `transport` | Device successfully connects |
| `device_disconnected` | — | Device disconnects |
| `disconnect_click` | — | User clicks disconnect button |

### Device Controls

| Event | Parameters | Trigger |
|---|---|---|
| `toggle` | `control`, `state` | Power, mute, or quiet couch toggled |
| `input_select` | `input` | Input source changed (hdmi/bluetooth/aux/optical) |
| `mode_select` | `mode` | Sound mode changed (movies/music/tv/news/manual) |
| `shape_select` | `shape` | Couch shape changed (straight/lshape/ushape/pit) |
| `slider_change` | `slider`, `value` | Volume, bass, treble, balance, center, or rear adjusted |
| `media_control` | `action` | Play/pause or skip track |

### Sound Profiles

| Event | Parameters | Trigger |
|---|---|---|
| `profile_create` | `name` | New sound profile saved |
| `profile_apply` | `name` | Existing profile activated |
| `profile_delete` | `name` | Profile deleted |

### Tip / Ko-fi

| Event | Parameters | Trigger |
|---|---|---|
| `tip_shown` | `action_count`, `times_dismissed` | Ko-fi tip tooltip becomes visible |
| `tip_dismissed` | `action_count`, `times_dismissed` | User closes the tip tooltip (X button) |
| `kofi_click` | `action_count`, `times_dismissed` | User clicks the Ko-fi link |

**Tip logic:** The tooltip first appears after 5 successful device actions (`FIRST_SHOW`). If dismissed, it re-appears after 25 more actions (`RESHOW_GAP`). Once the user clicks Ko-fi, it never shows again.

---

## GA4 Admin Setup

### Custom Dimensions

Register these at **Admin > Data Display > Custom Definitions > Create custom dimension**.

| Dimension Name | Scope | Event Parameter | Description |
|---|---|---|---|
| Control Name | Event | `control` | Which toggle was used (power, mute, quiet) |
| Toggle State | Event | `state` | Whether toggle was turned on or off |
| Input Source | Event | `input` | Selected input (hdmi, bluetooth, aux, optical) |
| Sound Mode | Event | `mode` | Selected sound mode (movies, music, tv, news, manual) |
| Couch Shape | Event | `shape` | Selected couch shape (straight, lshape, ushape, pit) |
| Slider Name | Event | `slider` | Which slider was adjusted (volume, bass, treble, balance, center-volume, rear-volume) |
| Transport Mode | Event | `transport` | Connection method (server or bluetooth) |
| Device Name | Event | `device_name` | Name of the connected StealthTech device |
| Profile Name | Event | `name` | Name of the sound profile |
| Media Action | Event | `action` | Media control action (play-pause, skip) |
| Card ID | Event | `card` | Which card section (connection, system, profiles, input, mode, volume, eq, shape, log) |
| Card Collapsed | Event | `collapsed` | Whether the card was collapsed (true) or expanded (false) |
| Theme | Event | `theme` | Selected theme (auto, light, dark) |
| Times Dismissed | Event | `times_dismissed` | Number of times the user has dismissed the Ko-fi tip |

### Custom Metrics

Register these at **Admin > Data Display > Custom Definitions > Custom metrics tab > Create custom metric**.

| Metric Name | Scope | Event Parameter | Unit | Description |
|---|---|---|---|---|
| Slider Value | Event | `value` | Standard | The value set on a slider control |
| Action Count | Event | `action_count` | Standard | Number of successful device actions at the time a tip event fires |

### Key Events (Conversions)

Mark these at **Admin > Data Display > Events** by toggling **Mark as key event**.

| Event | Why |
|---|---|
| `device_connected` | Core activation — user successfully paired with a device |
| `profile_create` | Power user engagement — user invested in customization |
| `kofi_click` | Monetization — user clicked through to support |
| `tip_shown` | Tip prompt reach — measures how many users see the ask |

> Key events won't appear in the list until they fire for the first time (may take 24–48 hours after deploy).

---

## Reports & Explorations

### 1. Feature Usage Overview

**Purpose:** Understand what features people actually use.

1. **Explore > Blank exploration**
2. Name: `Feature Usage Overview`
3. **Dimensions:** Event name
4. **Metrics:** Event count, Total users
5. **Filter:** Event name matches `toggle|input_select|mode_select|shape_select|slider_change|media_control`
6. **Visualization:** Table, sorted by Event count descending

### 2. Connection Funnel

**Purpose:** Track where users drop off between visiting the page and connecting.

1. **Explore > Funnel exploration**
2. Name: `Connection Funnel`
3. Steps:
   - Step 1: `page_view` (auto-collected by GA4)
   - Step 2: `transport_switch` (user picks a mode)
   - Step 3: `device_connected` (successful connection)
4. Shows drop-off rates at each stage

### 3. Input & Sound Mode Popularity

**Purpose:** Know which inputs and sound modes are most popular.

1. **Explore > Free form**
2. Name: `Input & Sound Mode Breakdown`
3. **Rows:** `input` dimension (filter to `input_select` events) and `mode` dimension (filter to `mode_select` events)
4. **Values:** Event count, Total users
5. Use segments or separate tabs to isolate input vs mode data

### 4. Transport Adoption

**Purpose:** Understand which connection method users prefer.

1. **Explore > Free form**
2. Name: `Transport Adoption`
3. **Dimensions:** `transport` (from `device_connected` events)
4. **Metrics:** Event count, Total users
5. **Visualization:** Pie chart or donut

### 5. Profile Engagement

**Purpose:** Track sound profile feature adoption.

1. **Explore > Free form**
2. Name: `Profile Engagement`
3. **Filter:** Event name matches `profile_create|profile_apply|profile_delete`
4. **Dimensions:** Event name
5. **Metrics:** Event count, Total users
6. Shows ratio of creates to applies to deletes

### 6. Session Engagement Depth

**Purpose:** How much do users do per session?

1. Use the built-in **Reports > Engagement > Events** report
2. Key metrics: events per session, average engagement time
3. No custom setup needed — GA4 tracks this automatically

### 7. Tip Conversion Funnel

**Purpose:** Measure whether the Ko-fi tip prompt converts to clicks.

1. **Explore > Funnel exploration**
2. Name: `Tip Conversion Funnel`
3. Steps:
   - Step 1: `tip_shown`
   - Step 2: `kofi_click` (conversion) or `tip_dismissed` (drop-off)
4. Shows the conversion rate from prompt to click
5. Break down by `times_dismissed` dimension to see if repeated prompts improve or reduce conversion

### 8. Tip Impact on Engagement

**Purpose:** Determine if showing the tip hurts user engagement (do users leave or stop using the app after seeing it?).

1. **Explore > Free form**
2. Name: `Tip Impact on Engagement`
3. Create two **Segments**:
   - **Saw Tip:** Users where `tip_shown` event count > 0
   - **Never Saw Tip:** Users where `tip_shown` event count = 0
4. **Metrics:** Average engagement time, Events per session, Sessions
5. **Visualization:** Comparison table
6. If "Saw Tip" users have lower engagement time or fewer return sessions, the tip prompt may be hurting retention

### 9. Tip Timing Analysis

**Purpose:** Find the optimal number of actions before showing the tip.

1. **Explore > Free form**
2. Name: `Tip Timing Analysis`
3. **Filter:** Event name = `kofi_click`
4. **Dimensions:** Action Count (custom metric), Times Dismissed (custom dimension)
5. **Metrics:** Event count
6. Interpretation:
   - If most conversions happen at action count ~5 (first show), `FIRST_SHOW = 5` is well tuned
   - If conversions increase after dismissals, the re-show logic is working
   - If very few convert after 2+ dismissals, consider capping the number of re-shows

---

## Debugging

To enable GA4 DebugView for real-time parameter inspection:

1. Temporarily add to `index.html`:
   ```js
   gtag('config', 'G-S8RFN5QZJH', { debug_mode: true });
   ```
2. Go to **Admin > Data Display > DebugView**
3. Events will appear in real-time with full parameter details
4. Remove `debug_mode` before deploying to production
