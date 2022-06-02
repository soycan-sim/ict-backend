function upload_resource() {
    let form = document.getElementById('upload-image');
    let image = form.elements.namedItem('image').files[0];

    image.arrayBuffer()
        .then(buffer => {
            let bytes = new Uint8Array(buffer);
            let binary = new String();
            for (b of bytes) {
                binary += String.fromCharCode(b);
            }
            return JSON.stringify({'mime': image.type, 'filename': image.name, 'data': btoa(binary)});
        })
        .then(body => fetch("/api/resource", {
            method: 'POST',
            body: body,
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
        }))
        .then(response => response.json())
        .then(data => {
            if (data.ok) {
                let form = document.getElementById('schedule-form');
                let image = form.elements.namedItem('image').value = data.filename;
            } else {
                console.log(data.error);
            }
        })
        .catch(err => console.log(err));
}
