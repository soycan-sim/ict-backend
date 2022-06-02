function set_cookie(cname, cvalue, exdays) {
  var d = new Date();
  d.setTime(d.getTime() + (exdays*24*60*60*1000));
  var expires = "expires="+ d.toUTCString();
  document.cookie = cname + "=" + cvalue + ";" + expires + ";path=/;SameSite=Strict";
}

function get_cookie(cname) {
  var name = cname + "=";
  var decoded_cookie = decodeURIComponent(document.cookie);
  var ca = decoded_cookie.split(';');
  for(var i = 0; i <ca.length; i++) {
    var c = ca[i];
    while (c.charAt(0) == ' ') {
      c = c.substring(1);
    }
    if (c.indexOf(name) == 0) {
      return c.substring(name.length, c.length);
    }
  }
  return "";
}

function check_cookie() {
    var notice = get_cookie("cookies_notice");
    if (notice == "") {
        alert("Diese Webseite verwendet Cookies für Ihre Sprachauswahl, Login Daten und Cookie-Einstellungen.\nMit der Benutzung der Website erklären Sie sich einverstanden mit der Speicherung der Cookies in Ihrem Browser.");
        set_cookie("cookies_notice", "1", 365);
    }
}

check_cookie();
