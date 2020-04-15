function make_formatted(fmt_left, fmt_right) {
    let elem = document.getElementById('editor-text-field');
    elem.focus();
    let start = elem.selectionStart;
    let end = elem.selectionEnd;
    let dir = elem.selectionDirection;
    if (dir != 'none') {
        let left = elem.value.substring(0, start);
        let middle = elem.value.substring(start, end);
        let right = elem.value.substring(end, elem.value.length);

        let content = null;
        if (middle.startsWith(fmt_left) && middle.endsWith(fmt_right)) {
            content = middle.substring(fmt_left.length, middle.length - fmt_right.length);

            elem.value = left + content + right;
            elem.selectionStart = start;
            elem.selectionEnd = end - fmt_left.length - fmt_right.length;
            elem.selectionDirection = 'forward';
        } else if (middle.startsWith(fmt_left)) {
            content = middle.substring(fmt_left.length);

            elem.value = left + content + right;
            elem.selectionStart = start;
            elem.selectionEnd = end - fmt_left.length;
            elem.selectionDirection = 'forward';
        } else if (middle.endsWith(fmt_right)) {
            content = middle.substring(0, middle.length - fmt_right.length);

            elem.value = left + content + right;
            elem.selectionStart = start;
            elem.selectionEnd = end - fmt_right.length;
            elem.selectionDirection = 'forward';
        } else {
            content = fmt_left + middle + fmt_right;

            elem.value = left + content + right;
            elem.selectionStart = start;
            elem.selectionEnd = end + fmt_left.length + fmt_right.length;
            elem.selectionDirection = 'forward';
        }

        let ev = new Event('input');
        elem.dispatchEvent(ev);
    }
}

function make_strong() {
    make_formatted('**', '**');
}

function make_emph() {
    make_formatted('_', '_');
}

function make_under() {
    make_formatted('<u>', '</u>');
}

function make_strike() {
    make_formatted('~~', '~~');
}

function load_draft() {
    let elem = document.getElementById('draft-select');

    if (elem.selectedOptions.length == 0) {
        return;
    }

    let id = elem.selectedOptions[0].value;
    let xhr = new XMLHttpRequest();
    xhr.open("GET", `/api/draft?id=${id}`, true);
    xhr.addEventListener('load', (pogress) => {
        let obj = JSON.parse(pogress.target.responseText);
        let title = obj['title'];
        let content = obj['content'];

        document.getElementById('editor-text-field').value = content;
        document.getElementById('title').value = title;

        let elem = document.getElementById('editor-text-field');

        let ev = new Event('input');
        elem.dispatchEvent(ev);
    })
    xhr.send();
}
