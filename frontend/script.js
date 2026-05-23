async function login() {

    const user =
        document.getElementById("user").value;

    const pass =
        document.getElementById("pass").value;

    const response = await fetch("/login", {
        method: "POST",

        headers: {
            "Content-Type": "application/json"
        },

        body: JSON.stringify({
            user,
            pass
        })
    });

    const text = await response.text();

    document.getElementById("msg").innerText =
        text;
}