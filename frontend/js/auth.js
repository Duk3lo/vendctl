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

async function logout() {
    try { await fetch("/api/logout", { method: "POST" }); } catch (e) { }
    window.location.href = "/login";
}