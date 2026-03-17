 function goTo(page){
    document.querySelectorAll('.page').forEach(p => p.classList.remove('pactive'));
    document.getElementById(page).classList.add('pactive');
    document.querySelectorAll('.pagebutton').forEach(pb => {
        pb.classList.remove('btn-primary', 'btn-outline-primary');
        if (pb.id !== page + "B") {
            pb.classList.add('btn-outline-primary');
        } else {
            pb.classList.add('btn-primary');
        }
    });
    // Re-fit the creator canvas now that it is visible
    if (page === 'Creator' && typeof fitCreatorCanvas === 'function') {
        fitCreatorCanvas();
    }
}
function encodeCode(code) {
    return encodeURIComponent(pako.deflate(code).toBase64({ alphabet: "base64", omitPadding: true }))
}
function decodeCode(code) {
    return pako.inflate(Uint8Array.fromBase64(decodeURIComponent(code)), { to: "string" })
}
function exp() {
    var code = encodeCode(getEditorContent())
    var url = location.origin + location.pathname + "?code=" + code
    navigator.clipboard.writeText(url)
    alert("Copied URL!")
}

var currentLine = 0;

function resetCurrentLine() {
    EDITOR.removeLineClass(currentLine, "background", "highlight-line");
    currentLine = 0;
    EDITOR.addLineClass(currentLine, "background", "highlight-line");
}

function nextLine() {
    if (currentLine != EDITOR.lineCount() - 1) {
        EDITOR.removeLineClass(currentLine, "background", "highlight-line");
        currentLine++;
        EDITOR.addLineClass(currentLine, "background", "highlight-line");
    }
}
function prevLine() {
    if (currentLine != 0 ) {
        EDITOR.removeLineClass(currentLine, "background", "highlight-line");
        currentLine--;
        EDITOR.addLineClass(currentLine, "background", "highlight-line");
    }
}