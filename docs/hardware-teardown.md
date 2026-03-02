# Lovesac StealthTech: a complete technical teardown of the hidden audio system

**Harman Kardon designs and manufactures virtually every audio component inside StealthTech**, confirmed through FCC filings, SEC disclosures, and Harman's own marketing materials. The system uses Summit Semiconductor's WiSA wireless modules for uncompressed 5 GHz audio between the center channel and subwoofer, Bluetooth Low Energy for app communication, and supports only Dolby Digital 5.1 and Dolby Pro Logic II—notably lacking DTS and Dolby Atmos. No public teardowns exist, but FCC internal photos, patent filings, open-source license disclosures, and community discussions collectively reveal the full technology stack powering this $3,700 embedded furniture audio system.

## Harman International builds the core audio hardware

**The primary hardware vendor is Harman International Industries** (the Harman Kardon brand), a wholly-owned subsidiary of Samsung Electronics since 2017. This is not speculation—it is confirmed across multiple authoritative sources. Harman filed the main FCC applications under grantee code **API** (FCC IDs: **APILOVESAC** and **APILOVESACR**), listing their Northridge, California headquarters as the manufacturer address. Every StealthTech audio component label reads "manufactured by Harman International Industries, Incorporated, 8500 Balboa Blvd, Northridge, CA 91329." Lovesac's own SEC 10-K and 10-Q filings consistently describe the product as featuring "immersive surround sound by Harman Kardon," and Harman's Embedded Audio division maintains a dedicated Lovesac partnership page.

The complete hardware lineup manufactured by Harman includes the center channel soundbar (model **EE4034**, rated at **242W RMS / 484W peak**), the active subwoofer receiver (model **EE0362**, 50W RMS with an 8-inch driver reaching down to 33 Hz), Sound+Charge sides for surround channels (models **GE0177/GE2913**, 52W RMS each with a 6.5-inch mid-woofer plus tweeter and a 3.5-inch mid with tweeter), and satellite surround sides (model **GE7065**, 12W RMS). All audio hardware is **manufactured in China** under Harman's supervision. Newer model numbers (GA4364, GA4956, GA4408, GR0692, GR5482) appearing in recent setup guides suggest ongoing hardware revisions.

Two additional vendors supply non-audio components. **jjPlus Corporation** of New Taipei City, Taiwan manufactures the Qi wireless charging pads (model QST008A, FCC ID **2A8R5QST008A**), filed directly by The Lovesac Company under grantee code **2A8R5**. The Bluetooth remote control (model EE3531) operates on Bluetooth 4.2 at 2.4 GHz with GFSK modulation.

## The WiSA wireless link and its silicon

The wireless audio backbone between the center channel and subwoofer/speakers is **WiSA (Wireless Speaker and Audio)**, not Bluetooth or standard Wi-Fi. This was confirmed by a former Lovesac store manager in App Store reviews and corroborated by FCC filings showing **Summit Semiconductor LLC** modules inside the subwoofer components.

The specific wireless module is Summit's model **444-2250** (FCC ID **UA9601**, IC **9129A-601**), found inside both the active subwoofer receiver (EE0362) and satellite subwoofer (EW3251). This module contains two key chips:

- **NXP MKW21D512** — an ARM Cortex-M4 MCU with an integrated IEEE 802.15.4 2.4 GHz radio transceiver, hardware MAC acceleration, and AES security, packaged in a 63-pin LGA system-in-package. This handles the control-plane signaling.
- **Summit SWM908 TX ASIC** — Summit's second-generation transmit ASIC with integrated analog front end and Dynamic Frequency Selection (DFS) circuitry, operating in the **5.15–5.825 GHz** band across 24 RF channels.

The WiSA protocol delivers **uncompressed 24-bit audio at 48/96 kHz** with a remarkably low **5.2 ms source-to-speaker latency**, supporting up to 7.1 channels. The module uses a patented quad-diversity PCB antenna design. The center channel transmits wirelessly to the subwoofer, which then distributes amplified audio via wired 6-pin and 3-pin cables to the surround speaker sides. Bluetooth (2.402–2.48 GHz, ~4.4 mW) serves two separate purposes: streaming music from phones/tablets and communicating with the StealthTech mobile app.

## Codec support is limited to Dolby Digital 5.1

StealthTech's audio processing supports only **Dolby Digital 5.1** and **Dolby Pro Logic II** (upmixing stereo to surround). The setup guide contains the required legal notice: "Manufactured under license from Dolby Laboratories." The system connects to TVs via **HDMI ARC** (not eARC), optical TOSLINK, 3.5mm AUX, or Bluetooth.

The absence of eARC is a critical technical constraint. ARC's limited bandwidth cannot carry lossless or object-based audio formats, which means **DTS, DTS-HD, DTS:X, and Dolby Atmos are all unsupported**. AVS Forum power users have pressed Lovesac on this, and while the company has reportedly indicated interest in adding Atmos via a firmware update, no such update has materialized as of early 2026. The system offers five Harman Kardon-tuned sound modes (Movie, TV, News, Music, and Quiet Couch Mode) and allows up to five custom EQ profiles adjustable via the app.

Two patented audio technologies differentiate StealthTech from conventional surround sound. **Fabric Tuning** adjusts EQ curves based on the specific upholstery fabric (over 200 options including microsuede, linen, velvet, leather) covering the speakers, accounting for each material's acoustic absorption characteristics. **Configuration Tuning** calibrates the surround sound field based on the physical layout of the modular Sactionals arrangement—straight, L-shaped, U-shaped, or pit configurations.

## Native mobile apps built by CitrusBits with persistent BLE issues

The StealthTech app was developed by **CitrusBits**, a mobile development agency, working in collaboration with Harman Kardon's hardware team. Both apps are **fully native**—the iOS version is written in **Swift** and the Android version in **Java/Kotlin**. They are emphatically not React Native or Flutter cross-platform builds.

Lovesac publishes an open-source license disclosure page at `lovesac.com/stealthtech-app-open-source-agreement`, which reveals the complete dependency stack. The iOS app uses **Alamofire** (5.4.3) for HTTP networking, **SwiftyBluetooth** (2.2.1) for CoreBluetooth BLE communication, and several UI libraries. The Android app uses **Retrofit** (2.9.0) and **OkHttp** (4.7.2) for networking, **BLESSED for BLE** (2.0.2) for Bluetooth communication, **Dagger** (2.28) for dependency injection, and **Glide** (4.11.0) for image loading.

The app communicates with the center channel exclusively via **Bluetooth Low Energy**, which has been the system's most persistent weak point. Chronic BLE disconnection issues—particularly on iOS—have plagued StealthTech since its 2021 launch, generating a torrent of 1-star App Store reviews. Lovesac's engineering team has acknowledged spending over six months diagnosing and correcting iPhone/Bluetooth connection problems. An AVS Forum user reported that Lovesac "changed manufacturer of the interface" (likely the Bluetooth module) to address early hardware-level connectivity failures while keeping the core Harman audio system intact. A Bluetooth chipset update released for units manufactured before January 2025 further confirms a hardware revision.

**Firmware updates are delivered via USB flash drive, not over-the-air.** Users must load four unzipped firmware files onto a FAT32/ExFAT-formatted USB drive, insert it into the center channel's Micro-USB port, and hold the remote's Mode button for 10–15 seconds. The firmware consists of three components: **MCU** (microcontroller), **APP** (application logic), and **EQ** (DSP/equalization). Known firmware versions progress from V-1.62 through V-1.66 (released May 2023) to the latest known V-1.71. This cumbersome update mechanism—unusual for a modern IoT product—means many owners likely run outdated firmware.

## No public source code, but the OSS disclosure reveals the stack

Searches across GitHub, GitLab, NPM, and PyPI returned **zero public repositories** containing StealthTech source code, SDKs, APIs, or firmware dumps. Lovesac maintains no public developer ecosystem. No security researchers have published decompiled app analyses or penetration testing results for StealthTech.

The open-source license disclosure page is the sole public window into the software stack. Beyond the BLE and networking libraries noted above, the iOS app includes KRProgressHUD (progress indicators), FAPanels (slide navigation), and Capable (accessibility management). The Android app uses Robolectric for unit testing, WheelPicker for scroll selection UI, and Timber for logging. The use of HTTP networking libraries (Alamofire, Retrofit/OkHttp) in both apps suggests they communicate with a Lovesac backend API—likely for telemetry, firmware version checking, or fabric/configuration database retrieval—even though primary device control occurs over BLE.

**No WiFi, AirPlay, Chromecast, or Spotify Connect support** exists. The system cannot integrate with home automation platforms. It functions only as a standalone audio system controllable via its physical remote or the BLE-connected app.

## FCC filings map the complete supply chain

Four distinct FCC registrations document StealthTech's component architecture:

| FCC ID | Grantee | Component | Key detail |
|---|---|---|---|
| **APILOVESAC** | Harman International (API) | Center channel Bluetooth transmitter | 2.4 GHz BLE, filed Aug 2021, revised Sep 2024 |
| **APILOVESACR** | Harman International (API) | Center channel DTS radio | Setup guide V48 on file |
| **2A8R5QST008A** | The Lovesac Company (2A8R5) | Qi wireless charging pad | 150–205 kHz, OEM: jjPlus Corp (Taiwan) |
| **UA9601** | Summit Semiconductor (UA9) | WiSA wireless audio module 444-2250 | 5.15–5.25 GHz, contained in subwoofer models |

Internal photos exist as PDF exhibits for the APILOVESAC filing (approximately 1.8 MB each, submitted with the September 2024 Class II Permissive Change application). These photos would reveal PCB layouts, DSP chips, amplifier ICs, and Bluetooth modules inside the center channel, but they are hosted as confidential or restricted-access documents on fccid.io. The September 2024 update—a Class II Permissive Change rather than a new filing—indicates a hardware revision to the Bluetooth subsystem while maintaining the same fundamental audio architecture.

## Eleven patents protect the embedded audio furniture concept

Lovesac holds **35 issued U.S. utility patents and 63 foreign utility patents** (per their FY2025 10-K filing), with 11 specifically listed under the StealthTech trademark. The foundational patent is **US 10,212,519 B2** ("Electronic furniture systems with integrated internal speakers"), filed November 2016 and granted February 2019, naming CEO Shawn Nelson alongside David Underwood, Brian Kuchler, David Cowan, and notably **Anthony Gallo**—an established speaker designer. This patent describes speakers embedded in modular furniture "transverse members" (armrests/sides) with subwoofers in the base, connected via selective couplers.

The most technically interesting patent is **US 10,972,838 B2** ("Electronic furniture systems with speaker tuning"), granted April 2021, which covers the fabric-specific and layout-specific audio calibration technology. A pending application (**US 20240000244 A1**) covers the acoustically transparent pillow design. Additional patents protect the integrated induction charging system (US 11,178,487, US 11,689,856, US 12,052,555) and an AI integration concept (US 10,979,241). The patent family has spawned over 15 continuation and continuation-in-part applications, with claims expiring between 2036 and 2042. International protection extends through PCT (WO 2021/141783), European (EP 4,088,365), and Canadian (CA 3,157,228) grants.

## No teardowns exist, but forum intelligence fills gaps

**No published teardown videos, iFixit guides, or reverse engineering blog posts** exist for StealthTech as of early 2026. The system is too niche and expensive to attract the typical teardown community. The closest substitute is the FCC internal photo exhibits, which remain the only publicly filed images of StealthTech's internal hardware.

The **AVS Forum "Lovesac Stealthtech owners thread"** is the single richest source of technical community intelligence. Key user-reported findings include: the subwoofer physically vibrates the seat cushion for a tactile bass effect; surround sound is "contained mostly to the couch" creating a localized listening bubble; rear channel volume is uneven depending on seating position; and the system performs well for casual movie watching but falls short of dedicated home theater systems at the same price point. Audiophile communities (Audioholics, QuadraphonicQuad) have been dismissive, comparing StealthTech to existing tactile transducer setups with "a ridiculous markup." Professional audio post-production house Smart Post Sound, which mixed the Dolby 5.1 showroom demos deployed across 300+ Lovesac locations, attested that "imaging remains clear, dynamics hold their balance, and the low end sits right where it should."

## Conclusion

StealthTech is fundamentally a **Harman Kardon 5.1 surround system** repackaged into modular furniture form factor, wirelessly linked via **Summit Semiconductor WiSA modules** running on NXP ARM Cortex-M4 silicon. The audio engineering is credible but constrained—Dolby Digital 5.1 through ARC only, no lossless formats, no network streaming. The software stack is native mobile (Swift/Kotlin) with BLE control, built by agency CitrusBits, and plagued by persistent connectivity issues that Lovesac has spent years addressing through both firmware and hardware revisions. The system's real innovation lies not in audio technology but in the **11+ patents covering acoustic integration with modular furniture**—fabric tuning, configuration calibration, and acoustically transparent materials. For potential integrators or competitors, the key takeaway is that StealthTech's moat is mechanical and design-patent-based, not audio-technological. The underlying audio and wireless components are commercially available platforms (Harman embedded audio, WiSA modules, standard Dolby licensing) assembled into a unique but technically modest package.