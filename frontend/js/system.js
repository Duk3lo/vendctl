async function updateSystemStatus() {
    try {
        const startPing = Date.now();
        const res = await fetch("/api/system/status");
        const localPingMs = Date.now() - startPing;
        if (handleAuthError(res) || !res.ok) return;

        const data = await res.json();
        document.getElementById("ping-local").innerText = localPingMs;
        document.getElementById("ping-discord").innerText = data.discord_ping || 0;

        if (data.wifi_connected) {
            currentConnectedSSID = data.wifi_ssid;
            let bars = data.wifi_rssi >= -60 ? 4 : data.wifi_rssi >= -70 ? 3 : data.wifi_rssi >= -80 ? 2 : 1;
            document.getElementById("sys-wifi-ssid").innerText = data.wifi_ssid;
            const statusEl = document.getElementById("sys-wifi-status");
            statusEl.innerText = data.has_internet ? "Señal: " + data.wifi_rssi + " dBm (Con Internet)" : "Señal: " + data.wifi_rssi + " dBm (Sin Internet)";
            statusEl.style.color = data.has_internet ? "#28a745" : "#ffc107";
            document.getElementById("home-wifi-icon").innerHTML = `<div class="sig-icon" style="width:30px; height:24px;">
                <div class="bar bar-1 ${bars >= 1 ? 'active' : ''}"></div><div class="bar bar-2 ${bars >= 2 ? 'active' : ''}"></div>
                <div class="bar bar-3 ${bars >= 3 ? 'active' : ''}"></div><div class="bar bar-4 ${bars >= 4 ? 'active' : ''}"></div>
            </div>`;
        } else {
            currentConnectedSSID = "";
            document.getElementById("sys-wifi-ssid").innerText = "Desconectado";
            document.getElementById("sys-wifi-status").innerText = "Buscando red...";
            document.getElementById("sys-wifi-status").style.color = "#ff5555";
            document.getElementById("home-wifi-icon").innerHTML = `<span style="font-size:24px; color:#ff5555;">❌</span>`;
        }

        if (data.ap_enabled) {
            document.getElementById("sys-ap-status").innerText = "Activada";
            document.getElementById("sys-ap-status").style.color = "#28a745";
            document.getElementById("ap-icon").style.opacity = "1";
            document.getElementById("sys-ap-ssid").innerText = "SSID: " + data.ap_ssid;
        } else {
            document.getElementById("sys-ap-status").innerText = "Desactivada";
            document.getElementById("sys-ap-status").style.color = "#ff5555";
            document.getElementById("ap-icon").style.opacity = "0.3";
            document.getElementById("sys-ap-ssid").innerText = "Apagada por sistema";
        }

        const dIcon = document.getElementById("discord-icon");
        if (data.discord_enabled) {
            document.getElementById("sys-discord-status").innerText = data.discord_running ? "Conectado" : "Conectando...";
            document.getElementById("sys-discord-status").style.color = data.discord_running ? "#28a745" : "#ffc107"; 
            document.getElementById("sys-discord-detail").innerText = data.discord_running ? "Operativo en Servidor" : "Iniciando WebSocket...";
            dIcon.style.opacity = data.discord_running ? "1" : "0.7";
        } else {
            document.getElementById("sys-discord-status").innerText = "Apagado";
            document.getElementById("sys-discord-status").style.color = "#ff5555"; 
            document.getElementById("sys-discord-detail").innerText = "Deshabilitado desde panel";
            dIcon.style.opacity = "0.3";
        }

        const physicalRamKb = 520;
        const dynTotalKb = Math.round(data.ram_total / 1024);
        const dynFreeKb = Math.round(data.ram_free / 1024);
        const dynUsedKb = dynTotalKb - dynFreeKb;
        const osReservedKb = physicalRamKb - dynTotalKb;

        document.getElementById("ram-free").innerText = dynFreeKb;
        document.getElementById("ram-os").innerText = osReservedKb;
        document.getElementById("ram-app").innerText = dynUsedKb;
        document.getElementById("ram-os-bar").style.width = (osReservedKb / physicalRamKb) * 100 + "%";
        document.getElementById("ram-app-bar").style.width = (dynUsedKb / physicalRamKb) * 100 + "%";
        document.getElementById("nvs-used").innerText = data.nvs_used;
        document.getElementById("nvs-total").innerText = data.nvs_total;
        document.getElementById("nvs-bar").style.width = (data.nvs_used / data.nvs_total) * 100 + "%";
    } catch (e) { }
}

// ARRANQUE AUTOMÁTICO
window.onload = function() {
    if (document.getElementById("ap-toggle")) {
        checkApStatus(); 
        loadSavedNetworks(); 
        loadApConfig(); 
        loadDiscordConfig(); 
    }

    if (document.getElementById("sys-wifi-ssid")) {
        updateSystemStatus(); 
        setInterval(updateSystemStatus, 3000);
    }
};