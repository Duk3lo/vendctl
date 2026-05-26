let selectedSSID = "";
let selectedAuth = "";
let isModalAdvOpen = false;
let currentConnectedSSID = "";

// --- VARIABLES DE DISCORD ---
let customCommands = [];
let slashCommands = [];
const MAX_COMMANDS_TOTAL = 10;

function handleAuthError(res) {
    if (res.status === 401) { window.location.href = "/login"; return true; }
    return false;
}

async function login() {
    const user = document.getElementById("user").value;
    const pass = document.getElementById("pass").value;
    const msg = document.getElementById("msg");

    if (msg) { msg.innerText = "Iniciando sesión..."; msg.style.color = "#aaa"; }

    try {
        const res = await fetch("/api/login", {
            method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ user, pass })
        });
        if (res.ok) window.location.href = "/dashboard";
        else if (msg) { msg.innerText = "Credenciales incorrectas"; msg.style.color = "#ff5555"; }
    } catch (e) {
        if (msg) { msg.innerText = "Error de conexión al servidor"; msg.style.color = "#ff5555"; }
    }
}

function showTab(id) {
    document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
    const targetTab = document.getElementById(id);
    if (targetTab) targetTab.classList.add('active');
    if (event && event.currentTarget) event.currentTarget.classList.add('active');
    if (id === 'tab-wifi') loadSavedNetworks();
}

async function loadSavedNetworks() {
    const list = document.getElementById("saved-list");
    if (!list) return;
    try {
        const res = await fetch("/api/wifi/saved");
        if (handleAuthError(res)) return;
        const nets = await res.json();
        list.innerHTML = nets.length === 0 ? "<li><small style='color:#888'>No hay redes guardadas.</small></li>" : "";
        nets.forEach(n => {
            const li = document.createElement("li");
            li.className = "wifi-item";
            li.innerHTML = `
                <div class="wifi-info">
                    <div><strong>${n.ssid}</strong><br><small style="color: #888;">${n.auth_type} ${n.user ? '| Usuario: ' + n.user : ''}</small></div>
                </div>
                <button class="danger" onclick="forgetNetwork('${n.ssid}')">Olvidar</button>
            `;
            list.appendChild(li);
        });
    } catch (e) { list.innerHTML = "<li><small style='color:red'>Error cargando redes.</small></li>"; }
}

async function forgetNetwork(ssid) {
    if (!confirm(`¿Eliminar la red ${ssid}?`)) return;
    const res = await fetch("/api/wifi/forget", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ssid }) });
    if (!handleAuthError(res)) loadSavedNetworks();
}

async function scanWifi() {
    const list = document.getElementById("wifi-list");
    if (!list) return;
    list.innerHTML = "<li>Buscando redes...</li>";
    try {
        const res = await fetch("/api/wifi/scan");
        if (handleAuthError(res)) return;
        const nets = await res.json();
        list.innerHTML = "";
        nets.forEach(n => {
            const li = document.createElement("li");
            li.className = "wifi-item";
            let bars = n.rssi >= -60 ? 4 : n.rssi >= -70 ? 3 : n.rssi >= -80 ? 2 : 1;
            let actionBtn = n.ssid === currentConnectedSSID ? `<button class="btn-disabled" disabled>Conectado</button>` : `<button onclick="openConnect('${n.ssid}', '${n.auth_method}')">Unirse</button>`;
            li.innerHTML = `
                <div class="wifi-info">
                    <div class="sig-icon">
                        <div class="bar bar-1 ${bars >= 1 ? 'active' : ''}"></div><div class="bar bar-2 ${bars >= 2 ? 'active' : ''}"></div>
                        <div class="bar bar-3 ${bars >= 3 ? 'active' : ''}"></div><div class="bar bar-4 ${bars >= 4 ? 'active' : ''}"></div>
                    </div>
                    <div><strong>${n.ssid}</strong><br><small style="color: #888;">${n.auth_method}</small></div>
                </div>${actionBtn}`;
            list.appendChild(li);
        });
    } catch (e) { list.innerHTML = "<li><small style='color:red'>Error al escanear.</small></li>"; }
}

function toggleModalAdvanced() {
    isModalAdvOpen = !isModalAdvOpen;
    document.getElementById("modal-advanced-fields").style.display = isModalAdvOpen ? "block" : "none";
    document.getElementById("adv-label").innerText = isModalAdvOpen ? "- Opciones avanzadas" : "+ Opciones avanzadas";
}

function openConnect(ssid, auth) {
    selectedSSID = ssid; selectedAuth = auth; isModalAdvOpen = false;
    document.getElementById("modal-advanced-fields").style.display = "none";
    document.getElementById("adv-label").innerText = "+ Opciones avanzadas";
    document.getElementById("modal-ssid").innerText = ssid;
    document.getElementById("pass-section").style.display = (auth === "None" || auth === "Open") ? "none" : "block";
    if (auth.includes("Enterprise")) toggleModalAdvanced();
    document.getElementById("wifi-pass").value = "";
    document.getElementById("wifi-user").value = "";
    document.getElementById("wifi-pass").type = "password";
    document.getElementById("modal").style.display = "flex";
}

function closeModal() { document.getElementById("modal").style.display = "none"; }

async function confirmConnect() {
    const body = {
        ssid: selectedSSID, pass: document.getElementById("wifi-pass").value, auth_type: selectedAuth,
        user: document.getElementById("wifi-user").value || null, anon_identity: null,
        eap_method: document.getElementById("wifi-eap").value, phase2: null
    };
    closeModal();
    const res = await fetch("/api/wifi/connect", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
    if (!handleAuthError(res) && res.ok) {
        alert("Conectando y guardando red.");
        setTimeout(loadSavedNetworks, 2000);
    }
}

async function checkApStatus() {
    const apToggle = document.getElementById("ap-toggle");
    if (!apToggle) return;
    try {
        const res = await fetch("/api/wifi/ap-status");
        if (!handleAuthError(res) && res.ok) apToggle.checked = (await res.json()).enabled;
    } catch (e) { }
}

async function toggleAP(checkbox) {
    await fetch("/api/wifi/ap-toggle", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ enable: checkbox.checked }) });
}

function toggleApPassword() {
    document.getElementById("ap-pass-section").style.display = document.getElementById("ap-config-open").value === "true" ? "none" : "block";
}

async function loadApConfig() {
    try {
        const res = await fetch("/api/wifi/ap-config");
        if (!handleAuthError(res) && res.ok) {
            const data = await res.json();
            document.getElementById("ap-config-ssid").value = data.ssid;
            document.getElementById("ap-config-open").value = data.open.toString();
            document.getElementById("ap-config-pass").value = data.pass;
            toggleApPassword();
        }
    } catch (e) { }
}

async function saveApConfig() {
    const ssid = document.getElementById("ap-config-ssid").value || "ESP32-SETUP";
    const pass = document.getElementById("ap-config-pass").value || "";
    const open = document.getElementById("ap-config-open").value === "true";
    if (!open && pass.length < 8) return alert("La contraseña debe tener al menos 8 caracteres");
    try {
        const res = await fetch("/api/wifi/ap-config", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ssid, pass, open }) });
        if (!handleAuthError(res) && res.ok) {
            alert("Zona Portátil actualizada.");
            document.getElementById("ap-toggle").checked = true;
            updateSystemStatus();
        }
    } catch (e) { alert("Error actualizando AP"); }
}

// --- LOGICA DEL BOT DE DISCORD ---
async function loadDiscordConfig() {
    try {
        const res = await fetch("/api/discord/config");
        if (!handleAuthError(res) && res.ok) {
            const data = await res.json();
            document.getElementById("discord-toggle").checked = data.enabled;
            document.getElementById("discord-token").value = data.token;
            document.getElementById("discord-appid").value = data.app_id;
            customCommands = data.custom_commands || [];
            slashCommands = data.slash_commands || [];
            renderCommands('text');
            renderCommands('slash');
        }
    } catch (e) { }
}

function renderCommands(type) {
    const listId = type === 'text' ? 'custom-commands-list' : 'slash-commands-list';
    const arr = type === 'text' ? customCommands : slashCommands;
    const list = document.getElementById(listId);
    
    list.innerHTML = "";
    arr.forEach((cmd, index) => {
        let placeholder = type === 'text' ? '!comando' : 'comando_slash';
        let extraOption = type === 'slash' ? `
            <div class="cmd-opt">
                <label style="font-size: 10px; color: #aaa; margin-bottom: 3px;">App</label>
                <input type="checkbox" style="width: 16px; height: 16px; margin: 0;" ${cmd.is_app_cmd ? "checked" : ""} onchange="updateCmd('${type}', ${index}, 'is_app_cmd', this.checked)">
            </div>` : "";

        list.innerHTML += `
            <div class="cmd-row">
                <input type="text" class="cmd-trigger" value="${cmd.trigger}" onchange="updateCmd('${type}', ${index}, 'trigger', this.value)" placeholder="${placeholder}">
                <textarea class="cmd-response" onchange="updateCmd('${type}', ${index}, 'response', this.value)" placeholder="Respuesta... [IMG:http...]">${cmd.response}</textarea>
                ${extraOption}
                <button class="danger cmd-delete" onclick="removeCommand('${type}', ${index})">X</button>
            </div>
        `;
    });
}

function addCommandRow(type) {
    if (customCommands.length + slashCommands.length >= MAX_COMMANDS_TOTAL) return alert("Límite de comandos alcanzado.");
    type === 'text' ? customCommands.push({ trigger: "!nuevo", response: "Respuesta...", is_app_cmd: false }) : slashCommands.push({ trigger: "nuevo", response: "Respuesta...", is_app_cmd: true });
    renderCommands(type);
}

function updateCmd(type, index, field, value) { type === 'text' ? customCommands[index][field] = value : slashCommands[index][field] = value; }
function removeCommand(type, index) { type === 'text' ? customCommands.splice(index, 1) : slashCommands.splice(index, 1); renderCommands(type); }

async function saveDiscordConfig() {
    const body = {
        enabled: document.getElementById("discord-toggle").checked, token: document.getElementById("discord-token").value,
        app_id: document.getElementById("discord-appid").value, custom_commands: customCommands, slash_commands: slashCommands
    };
    try {
        const res = await fetch("/api/discord/config", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
        if (!handleAuthError(res) && res.ok) alert("Configuración de Discord guardada.");
    } catch (e) { alert("Error guardando configuración."); }
}

async function logout() {
    try { await fetch("/api/logout", { method: "POST" }); } catch (e) { }
    window.location.href = "/login";
}

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

function togglePasswordVisibility(inputId) {
    const input = document.getElementById(inputId);
    input.type = input.type === "password" ? "text" : "password";
}

if (document.getElementById("ap-toggle")) {
    checkApStatus(); loadSavedNetworks(); loadApConfig(); loadDiscordConfig(); 
}

if (document.getElementById("sys-wifi-ssid")) {
    updateSystemStatus(); setInterval(updateSystemStatus, 3000);
}

function copyTag(text) { navigator.clipboard.writeText(text).then(() => { console.log("Copiado: " + text); }); }