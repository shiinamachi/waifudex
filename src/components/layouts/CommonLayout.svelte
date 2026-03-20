<script>
    import 'the-new-css-reset';
</script>

<slot />

<style>
    :global(:root) {
        color-scheme: light dark;
        background: Canvas;
        color: CanvasText;
    }

    :global(html),
    :global(body),
    :global(#app) {
        min-height: 100%;
        background: Canvas;
        color: CanvasText;
    }
</style>
