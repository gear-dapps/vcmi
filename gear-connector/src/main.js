const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event;

let addressInputEl;
let programIdInputEl;
let accountIdInputEl;
let passwordInputEl;

async function connect() {
  await invoke("connect", {
    programId: programIdInputEl.value,
    address: addressInputEl.innerText,
    accountId: accountIdInputEl.value,
    password: passwordInputEl.value
  });
}

window.addEventListener("DOMContentLoaded", () => {
  addressInputEl = document.querySelector("#address")
  programIdInputEl = document.querySelector("#program-id");
  accountIdInputEl = document.querySelector("#account-id")
  passwordInputEl = document.querySelector("#password")
  document
    .querySelector("#connect-button")
    .addEventListener("click", () => connect());

  document
    .querySelector("#skip-button")
    .addEventListener("click", () => invoke("skip"));
});

function readTextFile(file) {
  var rawFile = new XMLHttpRequest();
  rawFile.open("GET", file, false);
  var allText = ""
  rawFile.onreadystatechange = function () {
    if (rawFile.readyState === 4) {
      if (rawFile.status === 200 || rawFile.status == 0) {
        allText = rawFile.responseText;
      }
    }
  }
  rawFile.send(null);
  return allText
}

function createElementFromHTML(htmlString) {
  var div = document.createElement('div');
  div.innerHTML = htmlString.trim();

  // Change this to div.childNodes to support multiple top-level nodes.
  return div.firstChild;
}

await listen('alert', (event) => {
  console.log("js: connection_view: " + event)
  let programId = event.payload;

  let alert = document.getElementById("alert");
  alert.hidden = false

  document.getElementById("connection-message").innerText = "Program not found. Program ID:\n" + programId
})
