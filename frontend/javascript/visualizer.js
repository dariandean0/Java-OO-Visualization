import Module from '../../wasm/backend.js';
import * as Viz from 'https://cdn.jsdelivr.net/npm/@viz-js/viz@3.20.0/+esm'
import panzoom from 'https://cdn.jsdelivr.net/npm/panzoom@9.4.3/+esm'

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

    var execDotArray;
    async function update() {
        var dotCode = wasmVisualizeJavaCode(getEditorContent());
        var execDotCode = wasmExecFlowGen(getEditorContent());
        execDotArray = JSON.parse(execDotCode);
        Viz.instance().then(viz => {
            const svg = viz.renderSVGElement(dotCode);

            document.getElementById('GraphViewport').innerHTML = "";
            document.getElementById('GraphViewport').appendChild(svg);

            panzoom(svg, {
                maxZoom: 5,
                minZoom: 0.5,
                bounds: true,
                boundPadding: 0.05
            });

        });
        resetCurrentLine();
    }

    const debouncedUpdate = debounce(update, 500);

    EDITOR.on("change", debouncedUpdate);
    update();

    window.changeExecGraph = async function() {
        if(currentLine <= 0){ 
            update();
            return; 
        }
        if(currentLine-1 > execDotArray.length){ return; }

        Viz.instance().then(viz => {
            const svg = viz.renderSVGElement(execDotArray[currentLine-1]);

            document.getElementById('GraphViewport').innerHTML = "";
            document.getElementById('GraphViewport').appendChild(svg);

            panzoom(svg, {
                maxZoom: 5,
                minZoom: 0.5,
                bounds: true,
                boundPadding: 0.05
            });

        });
    }
})();

// update from URL input
const urlParams = new URLSearchParams(window.location.search)
const codeParam = urlParams.get('code')
if (codeParam) {
    EDITOR.setValue(decodeCode(codeParam))
}