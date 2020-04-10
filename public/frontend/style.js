if (window['l10n_cache'] === undefined) {
    window['l10n_cache'] = {}
}

togglenav = {
    hidden: false,
    width: undefined,
};

function l10n(which, onload) {
    if (l10n_cache[which] === undefined) {
        let xhr = new XMLHttpRequest();
        xhr.addEventListener('load', (ev) => {
            let obj = JSON.parse(ev.target.responseText);
            let t9n = obj['t9n'];
            l10n_cache[which] = t9n;
            onload(t9n);
        });
        xhr.open("GET", `/api/t9n?which=${which}`, true);
        xhr.send();
    } else {
        onload(l10n_cache[which]);
    }
}

function toggleNav(id) {
    if (togglenav.hidden) {
        togglenav.hidden = false;
        document.getElementById(id).style.width = togglenav.width;

        l10n('hide', (t9n) => {
            document.getElementById('togglenav').innerHTML = t9n;
        });
    } else {
        togglenav.width = document.getElementById(id).style.width;
        togglenav.hidden = true;
        document.getElementById(id).style.width = '10px';

        l10n('show', (t9n) => {
            document.getElementById('togglenav').innerHTML = t9n;
        });
    }
    return false;
}
