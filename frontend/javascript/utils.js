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