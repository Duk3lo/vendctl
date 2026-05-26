let customCommands = [];
let slashCommands = [];
const MAX_COMMANDS_TOTAL = 10;

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