document.addEventListener("keypress", event => {
    if (event.key == "w" && document.activeElement.tagName == "BODY") {
        document.querySelectorAll("pre").forEach(el => {
            if (el.style.whiteSpace != "pre") {
                el.style.whiteSpace = "pre";
            } else {
                el.style.whiteSpace = "pre-wrap";
            }
        });
    }
})