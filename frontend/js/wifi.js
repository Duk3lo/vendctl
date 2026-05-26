let selectedSSID = "";
let selectedAuth = "";
let currentConnectedSSID = "";

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
            if(typeof updateSystemStatus === "function") updateSystemStatus();
        }
    } catch (e) { alert("Error actualizando AP"); }
}