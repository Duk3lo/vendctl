let selectedSSID = "";
let selectedAuth = "";
let isModalAdvOpen = false;
let currentConnectedSSID = "";

// --- VARIABLES DE DISCORD ---
let customCommands = [];
let slashCommands = [];
const MAX_COMMANDS_TOTAL = 10; // Para no agotar la RAM de ESP32

function handleAuthError(res) {
    if (res.status === 401) {
        window.location.href = "/login";
        return true;
    }
    return false;
}

async function login() {
    const user = document.getElementById("user").value;
    const pass = document.getElementById("pass").value;
    const msg = document.getElementById("msg");

    if (msg) { msg.innerText = "Iniciando sesión..."; msg.style.color = "#aaa"; }

    try {
        const res = await fetch("/api/login", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ user, pass })
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
        list.innerHTML = "";
        if (nets.length === 0) {
            list.innerHTML = "<li><small style='color:#888'>No hay redes guardadas.</small></li>";
            return;
        }
        nets.forEach(n => {
            const li = document.createElement("li");
            li.className = "wifi-item";
            li.innerHTML = `
                <div class="wifi-info">
                    <div>
                        <strong>${n.ssid}</strong><br>
                        <small style="color: #888;">${n.auth_type} ${n.user ? '| Usuario: ' + n.user : ''}</small>
                    </div>
                </div>
                <button class="danger" onclick="forgetNetwork('${n.ssid}')">Olvidar</button>
            `;
            list.appendChild(li);
        });
    } catch (e) {
        list.innerHTML = "<li><small style='color:red'>Error cargando redes.</small></li>";
    }
}

async function forgetNetwork(ssid) {
    if (!confirm(`¿Eliminar la red ${ssid}?`)) return;
    const res = await fetch("/api/wifi/forget", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ssid })
    });
    if (handleAuthError(res)) return;
    loadSavedNetworks();
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

            let bars = 1;
            if (n.rssi >= -60) bars = 4;
            else if (n.rssi >= -70) bars = 3;
            else if (n.rssi >= -80) bars = 2;

            let actionBtn = "";
            if (n.ssid === currentConnectedSSID) {
                actionBtn = `<button class="btn-disabled" disabled>Conectado</button>`;
            } else {
                actionBtn = `<button onclick="openConnect('${n.ssid}', '${n.auth_method}')">Unirse</button>`;
            }

            li.innerHTML = `
                <div class="wifi-info">
                    <div class="sig-icon">
                        <div class="bar bar-1 ${bars >= 1 ? 'active' : ''}"></div>
                        <div class="bar bar-2 ${bars >= 2 ? 'active' : ''}"></div>
                        <div class="bar bar-3 ${bars >= 3 ? 'active' : ''}"></div>
                        <div class="bar bar-4 ${bars >= 4 ? 'active' : ''}"></div>
                    </div>
                    <div>
                        <strong>${n.ssid}</strong><br>
                        <small style="color: #888;">${n.auth_method}</small>
                    </div>
                </div>
                ${actionBtn}
            `;
            list.appendChild(li);
        });
    } catch (e) {
        list.innerHTML = "<li><small style='color:red'>Error al escanear.</small></li>";
    }
}

function toggleModalAdvanced() {
    isModalAdvOpen = !isModalAdvOpen;
    document.getElementById("modal-advanced-fields").style.display = isModalAdvOpen ? "block" : "none";
    document.getElementById("adv-label").innerText = isModalAdvOpen ? "- Opciones avanzadas" : "+ Opciones avanzadas";
}

function openConnect(ssid, auth) {
    selectedSSID = ssid;
    selectedAuth = auth;
    isModalAdvOpen = false;
    document.getElementById("modal-advanced-fields").style.display = "none";
    document.getElementById("adv-label").innerText = "+ Opciones avanzadas";
    document.getElementById("modal-ssid").innerText = ssid;

    const isOpen = auth === "None" || auth === "Open";
    document.getElementById("pass-section").style.display = isOpen ? "none" : "block";

    if (auth.includes("Enterprise")) toggleModalAdvanced();

    document.getElementById("wifi-pass").value = "";
    document.getElementById("wifi-user").value = "";
    document.getElementById("wifi-anon").value = "";
    document.getElementById("wifi-pass").type = "password";

    document.getElementById("modal").style.display = "flex";
}

function closeModal() { document.getElementById("modal").style.display = "none"; }

async function confirmConnect() {
    const body = {
        ssid: selectedSSID, pass: document.getElementById("wifi-pass").value, auth_type: selectedAuth,
        user: document.getElementById("wifi-user").value || null, anon_identity: document.getElementById("wifi-anon").value || null,
        eap_method: document.getElementById("wifi-eap").value, phase2: document.getElementById("wifi-phase2").value
    };
    closeModal();
    const res = await fetch("/api/wifi/connect", {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body)
    });
    if (handleAuthError(res)) return;

    if (res.ok) {
        alert("Conectando y guardando red. (El AP se apagará automáticamente al conectar).");
        setTimeout(loadSavedNetworks, 2000);
    }
}

async function checkApStatus() {
    const apToggle = document.getElementById("ap-toggle");
    if (!apToggle) return;
    try {
        const res = await fetch("/api/wifi/ap-status");
        if (handleAuthError(res)) return;

        if (res.ok) {
            const data = await res.json();
            apToggle.checked = data.enabled;
        }
    } catch (e) { }
}

async function toggleAP(checkbox) {
    const enable = checkbox.checked;
    const res = await fetch("/api/wifi/ap-toggle", {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ enable })
    });
    handleAuthError(res);
}

function toggleApPassword() {
    const isOpen = document.getElementById("ap-config-open").value === "true";
    document.getElementById("ap-pass-section").style.display = isOpen ? "none" : "block";
}

async function loadApConfig() {
    try {
        const res = await fetch("/api/wifi/ap-config");
        if (handleAuthError(res)) return;

        if (res.ok) {
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

    if (!open && pass.length < 8) {
        alert("La contraseña debe tener al menos 8 caracteres");
        return;
    }

    try {
        const res = await fetch("/api/wifi/ap-config", {
            method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ssid, pass, open })
        });
        if (handleAuthError(res)) return;

        if (res.ok) {
            alert("Zona Portátil actualizada y guardada.");
            document.getElementById("ap-toggle").checked = true;
            updateSystemStatus();
        }
    } catch (e) {
        alert("Error actualizando AP");
    }
}

// --- LOGICA DEL BOT DE DISCORD ---

async function loadDiscordConfig() {
    try {
        const res = await fetch("/api/discord/config");
        if (handleAuthError(res)) return;
        if (res.ok) {
            const data = await res.json();
            document.getElementById("discord-toggle").checked = data.enabled;
            document.getElementById("discord-token").value = data.token;
            document.getElementById("discord-appid").value = data.app_id;
            
            // Cargar ambos arrays
            customCommands = data.custom_commands || [];
            slashCommands = data.slash_commands || [];
            
            // Renderizar ambas listas
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
        
        // NUEVO: Agregamos un checkbox si es Slash Command
        let extraOption = "";
        if (type === 'slash') {
            let isChecked = cmd.is_app_cmd ? "checked" : "";
            extraOption = `
                <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; min-width: 60px;">
                    <label style="font-size: 10px; color: #aaa; margin-bottom: 3px;">Global (App)</label>
                    <input type="checkbox" style="width: 16px; height: 16px; margin: 0;" ${isChecked} 
                        onchange="updateCmd('${type}', ${index}, 'is_app_cmd', this.checked)">
                </div>
            `;
        }

        list.innerHTML += `
            <div style="display: flex; gap: 10px; margin-bottom: 10px; align-items: center; background: #252525; padding: 8px; border-radius: 6px; border: 1px solid #333;">
                <input type="text" value="${cmd.trigger}" onchange="updateCmd('${type}', ${index}, 'trigger', this.value)" placeholder="${placeholder}" style="width: 30%; border:none; background:#1e1e1e;">
                <input type="text" value="${cmd.response}" onchange="updateCmd('${type}', ${index}, 'response', this.value)" placeholder="Respuesta..." style="flex: 1; border:none; background:#1e1e1e;">
                ${extraOption}
                <button class="danger" style="padding: 6px 12px;" onclick="removeCommand('${type}', ${index})">X</button>
            </div>
        `;
    });
}

function addCommandRow(type) {
    if (customCommands.length + slashCommands.length >= MAX_COMMANDS_TOTAL) {
        alert("Límite de comandos alcanzado (Max 10) para proteger la RAM del ESP32.");
        return;
    }
    
    if (type === 'text') {
        customCommands.push({ trigger: "!nuevo", response: "Respuesta...", is_app_cmd: false });
    } else {
        slashCommands.push({ trigger: "nuevo", response: "Respuesta...", is_app_cmd: true });
    }
    renderCommands(type);
}

function updateCmd(type, index, field, value) {
    if (type === 'text') {
        customCommands[index][field] = value;
    } else {
        slashCommands[index][field] = value;
    }
}

function removeCommand(type, index) {
    if (type === 'text') {
        customCommands.splice(index, 1);
    } else {
        slashCommands.splice(index, 1);
    }
    renderCommands(type);
}

async function saveDiscordConfig() {
    const body = {
        enabled: document.getElementById("discord-toggle").checked,
        token: document.getElementById("discord-token").value,
        app_id: document.getElementById("discord-appid").value,
        custom_commands: customCommands,
        slash_commands: slashCommands
    };

    try {
        const res = await fetch("/api/discord/config", {
            method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body)
        });
        if (handleAuthError(res)) return;
        if (res.ok) alert("Configuración de Discord guardada. Los cambios se aplican automáticamente.");
    } catch (e) {
        alert("Error guardando configuración de Discord.");
    }
}

// ---------------------------------

async function logout() {
    try { await fetch("/api/logout", { method: "POST" }); } catch (e) { }
    window.location.href = "/login";
}

async function updateSystemStatus() {
    try {
        const startPing = Date.now(); // Inicio del contador de Ping Local
        const res = await fetch("/api/system/status");
        const localPingMs = Date.now() - startPing; // Fin del contador

        if (handleAuthError(res)) return;

        if (res.ok) {
            const data = await res.json();
            
            // --- ACTUALIZACIÓN DE PINGS EN TIEMPO REAL ---
            document.getElementById("ping-local").innerText = localPingMs;
            document.getElementById("ping-discord").innerText = data.discord_ping || 0;

            if (data.wifi_connected) {
                currentConnectedSSID = data.wifi_ssid;
                let bars = 1;
                if (data.wifi_rssi >= -60) bars = 4;
                else if (data.wifi_rssi >= -70) bars = 3;
                else if (data.wifi_rssi >= -80) bars = 2;

                document.getElementById("sys-wifi-ssid").innerText = data.wifi_ssid;

                const statusEl = document.getElementById("sys-wifi-status");
                if (data.has_internet) {
                    statusEl.innerText = "Señal: " + data.wifi_rssi + " dBm (Con Internet)";
                    statusEl.style.color = "#28a745";
                } else {
                    statusEl.innerText = "Señal: " + data.wifi_rssi + " dBm (Sin Internet)";
                    statusEl.style.color = "#ffc107";
                }

                document.getElementById("home-wifi-icon").innerHTML = `
                    <div class="sig-icon" style="width:30px; height:24px;">
                        <div class="bar bar-1 ${bars >= 1 ? 'active' : ''}"></div>
                        <div class="bar bar-2 ${bars >= 2 ? 'active' : ''}"></div>
                        <div class="bar bar-3 ${bars >= 3 ? 'active' : ''}"></div>
                        <div class="bar bar-4 ${bars >= 4 ? 'active' : ''}"></div>
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

            // --- ACTUALIZACIÓN ESTADO DISCORD ---
            const dIcon = document.getElementById("discord-icon");
            if (data.discord_enabled) {
                // Verificamos si el Hilo de Rust está ejecutando el bot activamente
                if (data.discord_running) {
                    document.getElementById("sys-discord-status").innerText = "Conectado";
                    document.getElementById("sys-discord-status").style.color = "#28a745"; // Verde
                    document.getElementById("sys-discord-detail").innerText = "Operativo en Servidor";
                    dIcon.style.opacity = "1";
                } else {
                    document.getElementById("sys-discord-status").innerText = "Conectando...";
                    document.getElementById("sys-discord-status").style.color = "#ffc107"; // Amarillo
                    document.getElementById("sys-discord-detail").innerText = "Iniciando WebSocket...";
                    dIcon.style.opacity = "0.7";
                }
            } else {
                document.getElementById("sys-discord-status").innerText = "Apagado";
                document.getElementById("sys-discord-status").style.color = "#ff5555"; // Rojo
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
        }
    } catch (e) { }
}

function togglePasswordVisibility(inputId) {
    const input = document.getElementById(inputId);
    if (input.type === "password") {
        input.type = "text";
    } else {
        input.type = "password";
    }
}

// --- INICIALIZADORES AL CARGAR LA PÁGINA ---
if (document.getElementById("ap-toggle")) {
    checkApStatus();
    loadSavedNetworks();
    loadApConfig();
    loadDiscordConfig(); // Carga la config de discord
}

if (document.getElementById("sys-wifi-ssid")) {
    updateSystemStatus();
    setInterval(updateSystemStatus, 3000);
}

function copyTag(text) {
    navigator.clipboard.writeText(text).then(() => {
        console.log("Copiado al portapapeles: " + text);
    });
}