const preview = document.getElementById('preview-image');
const caption = document.getElementById('preview-caption');
for (const button of document.querySelectorAll('[data-image]')) {
  button.addEventListener('click', () => {
    preview.src = 'assets/' + button.dataset.image;
    preview.alt = button.dataset.caption;
    caption.textContent = button.dataset.caption;
    for (const other of document.querySelectorAll('[data-image]')) {
      const active = other === button;
      other.classList.toggle('active', active);
      other.setAttribute('aria-pressed', String(active));
    }
  });
}
