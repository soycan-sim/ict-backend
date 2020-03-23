togglenav = {
    hidden: false,
    width: undefined,
};

function toggleNav(id) {
    if (togglenav.hidden) {
        togglenav.hidden = false;
        document.getElementById(id).style.width = togglenav.width;
        document.getElementById('togglenav').innerHTML = 'Hide';
    } else {
        togglenav.width = document.getElementById(id).style.width;
        togglenav.hidden = true;
        document.getElementById(id).style.width = '10px';
        document.getElementById('togglenav').innerHTML = 'Show';
    }
    return false;
}
