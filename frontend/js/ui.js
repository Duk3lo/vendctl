let isModalAdvOpen = false;

function showTab(id) {
    document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
    const targetTab = document.getElementById(id);
    if (targetTab) targetTab.classList.add('active');
    if (event && event.currentTarget) event.currentTarget.classList.add('active');
    
    // Si entramos al WiFi, cargamos la lista
    if (id === 'tab-wifi' && typeof loadSavedNetworks === "function") loadSavedNetworks();
}

function toggleModalAdvanced() {
    isModalAdvOpen = !isModalAdvOpen;
    document.getElementById("modal-advanced-fields").style.display = isModalAdvOpen ? "block" : "none";
    document.getElementById("adv-label").innerText = isModalAdvOpen ? "- Opciones avanzadas" : "+ Opciones avanzadas";
}

function closeModal() { 
    document.getElementById("modal").style.display = "none"; 
}

function togglePasswordVisibility(inputId) {
    const input = document.getElementById(inputId);
    input.type = input.type === "password" ? "text" : "password";
}

function copyTag(text) { 
    navigator.clipboard.writeText(text).then(() => { console.log("Copiado: " + text); }); 
}