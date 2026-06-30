document.body.addEventListener("htmx:afterRequest", function (event) {
    if (event.detail.successful) {
        const form = event.detail.elt;
        if (form && form.tagName === "FORM") {
            form.reset();
        }
    }
});
