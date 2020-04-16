function make_admin(ev, id) {
    let value = ev.checked;

    let xhr = new XMLHttpRequest();
    xhr.open("POST", "/api/setadmin", true);
    xhr.addEventListener('load', (pogress) => {
        console.log(JSON.parse(pogress.target.responseText));
    });
    xhr.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
    xhr.send(`value=${value}&uid=${id}`);
}

function make_employee(ev, id) {
    let value = ev.checked;

    let xhr = new XMLHttpRequest();
    xhr.open("POST", "/api/setemployee", true);
    xhr.addEventListener('load', (pogress) => {
        console.log(JSON.parse(pogress.target.responseText));
    });
    xhr.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
    xhr.send(`value=${value}&uid=${id}`);
}
