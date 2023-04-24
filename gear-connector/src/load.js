const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event;

const rows = document.querySelectorAll('.table-row');

rows.forEach(row => {
  row.addEventListener('click', () => {
    rows.forEach(row => {
      row.classList.remove('table-active');
    });
    row.classList.add('table-active');
  });
});