import Module from '../../wasm/backend.js';
import * as Viz from 'https://cdn.jsdelivr.net/npm/@viz-js/viz@3.20.0/+esm'

(async () => {
    const mod = await Module();

    const wasmExecFlowGen = mod.cwrap(
        'wasm_execution_flow_gen',
        'string',
        ['string']
    );

    const wasmNoFlowGen = mod.cwrap(
        'wasm_no_flow_gen',
        'string',
        ['string']
    );

    const wasmVisualizeJavaCode = mod.cwrap(
        'wasm_visualize_java_code',
        'string',
        ['string']
    );



    //VizJS live update

    function debounce(fn, delay) {
        var timeout;
        return (...args) => {
            clearTimeout(timeout);
            timeout = setTimeout(() => fn(...args), delay);
        };
    }

    async function update() {
        var dotCode = wasmVisualizeJavaCode(getEditorContent())
        //var dotCode = wasmExecFlowGen(getEditorContent());
        console.log(dotCode);

        Viz.instance().then(viz => {
            const svg = viz.renderSVGElement(dotCode);

            svg.removeAttribute("width");
            svg.removeAttribute("height");
            svg.style.width = "100%";
            svg.style.height = "auto";

            document.getElementById('Graph').innerHTML = "<div class='p-2 fw-bold' style='background-color: #DDDDDD;'>Memory Diagram</div>"; //What?
            document.getElementById('Graph').appendChild(svg);
        });
    }

    const debouncedUpdate = debounce(update, 500);

    EDITOR.on("change", debouncedUpdate);
})();

// update from URL input
const urlParams = new URLSearchParams(window.location.search)
const codeParam = urlParams.get('code')
if (codeParam) {
    EDITOR.setValue(decodeCode(codeParam))
}