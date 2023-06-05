const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event;

let lobbyAddressInputEl;
let usernameInputEl;
let nodeAddressInputEl;
let programIdInputEl;
let accountIdInputEl;
let passwordInputEl;

let roomNameEl;
let roomPasswordEl;
let roomMaxPlayersEl;

async function connect() {
  await invoke("connect", {
    lobbyAddress: lobbyAddressInputEl.value,
    username: usernameInputEl.value,
    programId: programIdInputEl.value,
    nodeAddress: nodeAddressInputEl.innerText,
    accountId: accountIdInputEl.value,
    password: passwordInputEl.value
  });
}

//create new room
//%1: room name
//%2: password for the room
//%3: max number of players
//%4: mods used by host
// each mod has a format modname&modversion, mods should be separated by ; symbol
// {CREATE, "<NEW>%1<PSWD>%2<COUNT>%3<MODS>%4"},

async function newRoom() {
  console.log(roomMaxPlayersEl.innerText, roomMaxPlayersEl.value)
  console.log("NewRoom", roomNameEl.value, roomPasswordEl.value, roomMaxPlayersEl.value)
  await invoke("new_room", {
    roomName: roomNameEl.value,
    password: roomPasswordEl.value,
    maxPlayers: parseInt(roomMaxPlayersEl.innerText),
    mods: "h3-for-vcmi-englisation&1.2;vcmi&1.2;vcmi-extras&3.3.6;vcmi-extras.arrowtowericons&1.1;vcmi-extras.battlefieldactions&0.2;vcmi-extras.bonusicons&0.8.1;vcmi-extras.bonusicons.bonus icons&0.8;vcmi-extras.bonusicons.immunity icons&0.6;vcmi-extras.extendedrmg&1.2;vcmi-extras.extraresolutions&1.0;vcmi-extras.quick-exchange&1.0"
  });
}

async function joinRoom(roomName) {
  console.log("JOIN ROOM", roomName);
  await invoke("join_room", {
    roomName: roomName,
    password: "",
    mods: ""
  });
}

async function ready(roomName) {
  console.log("ready", roomName);
  await invoke("ready", {
    roomName: roomName,
  });
}

async function leave(roomName) {
  console.log("leave", roomName);
  await invoke("leave", {
    roomName: roomName,
  });
}

async function hostmode(mode) {
  console.log("hostmode", mode);
  await invoke("hostmode", {
    mode: mode,
  });
}

window.addEventListener("DOMContentLoaded", () => {
  lobbyAddressInputEl = document.querySelector("#lobby-address")
  usernameInputEl = document.querySelector("#username")
  nodeAddressInputEl = document.querySelector("#node-address")
  programIdInputEl = document.querySelector("#program-id");
  accountIdInputEl = document.querySelector("#account-id")
  passwordInputEl = document.querySelector("#password")
  document.querySelector("#connect-button").addEventListener("click", () => connect());

  roomNameEl = document.querySelector("#room-name")
  roomPasswordEl = document.querySelector("#room-password")
  roomMaxPlayersEl = document.querySelector("#room-max-players")
  document.querySelector("#new-room-button").addEventListener("click", () => newRoom());
});

await listen('alert', (event) => {
  console.log("js: connection_view: " + event)
  let programId = event.payload;

  let alert = document.getElementById("alert");
  alert.hidden = false

  document.getElementById("connection-message").innerText = "Program not found. Program ID:\n" + programId
})

await listen('showRooms', (event) => {
  let roomView = document.getElementById("collapseRoom");
  let bsCollapse = new bootstrap.Collapse(roomView);
  console.log("show Rooms:", users);
  bsCollapse.toggle();
  document.getElementById("connect-button").hidden = true
})

await listen('addUsers', (event) => {
  let users = event.payload;
  console.log("add Users:", users);
  const list = document.getElementById("users");
  while (list.firstChild) {
    list.removeChild(list.firstChild);
  }
  for (let i = 0; i < users.length; ++i) {
    const listItem = document.createElement("li");
    listItem.className = "list-group-item";
    listItem.classList.add("bg-secondary");
    // listItem.classList.add("border-primary-subtle")

    listItem.style = "--bs-bg-opacity: .2;"
    listItem.textContent = users[i];
    list.appendChild(listItem);
  }
})

await listen('addSessions', (event) => {
  let sessions = event.payload;
  console.log("add Session:", sessions);
  const div = document.getElementById("sessions");
  while (div && div.firstChild !== null) {
    div.removeChild(div.firstChild)
  }
  for (let i = 0; i < sessions.length; ++i) {
    const button = document.createElement("button");
    button.classList.add("btn")
    button.classList.add("btn-secondary")
    // button.classList.add("border-primary")
    // button.style = "background-color: rgb(0, 0, 50);"
    button.textContent = sessions[i].name
    button.value = sessions[i].name
    button.addEventListener("click", () => joinRoom(button.value));

    const badge = document.createElement("span");
    badge.classList.add("badge")
    badge.classList.add("text-bg-light")
    badge.classList.add("rounded-pill")
    badge.textContent = sessions[i].joined + "/" + sessions[i].total
    button.appendChild(badge)

    div.appendChild(button)
  }
})

await listen('chatMessage', (event) => {
  let messages = event.payload;
  console.log("chat Message:", messages);
  const list = document.getElementById("messages");
  const listItem = document.createElement("li");

  listItem.className = "list-group-item";
  const strong = document.createElement("strong");
  strong.textContent = messages[0] + ": ";
  listItem.appendChild(strong);
  const text = document.createElement("text");
  text.textContent = messages[1];
  listItem.appendChild(text);

  listItem.classList.add("bg-dark");

  listItem.style = "--bs-bg-opacity: .2;"
  list.appendChild(listItem);
})

await listen('created', (event) => {
  let room_name = event.payload;
  console.log("created room:", room_name);
  const modalElement = document.getElementById("roomModal");
  modalElement.classList.remove("show");
  modalElement.style.display = "none";
  modalElement.setAttribute("aria-hidden", "true");
  modalElement.removeAttribute("aria-modal");
  const modalBackdrop = document.getElementsByClassName("modal-backdrop")[0];
  modalBackdrop.parentNode.removeChild(modalBackdrop);
  document.body.classList.remove("modal-open");
})

await listen('status', (event) => {
  const usersCount = event.payload[0]
  const players = document.getElementById("players")
  const statuses = event.payload[1]
  while (players.firstChild) players.removeChild(players.firstChild);

  for (let i = 0; i < usersCount; ++i) {
    const listItem = document.createElement("li");
    const status = statuses[i];


    listItem.className = "list-group-item";
    if (status[1] === 'True') {
      const div = document.createElement("div");
      div.classList.add("form-check")

      const check = document.createElement("input");
      check.setAttribute("checked", "")
      check.setAttribute("type", "checkbox")
      check.classList.add("form-check-input");
      check.classList.add("form-check-input");
      check.style = "background-color: #28a745; border-color: #28a745;"
      check.id = status[0] + i
      div.appendChild(check)

      const label = document.createElement("label");
      label.classList.add("form-check-label");
      label.setAttribute("for", check.id)
      label.textContent = status[0]
      div.appendChild(label)

      listItem.classList.add("bg-success");
      listItem.classList.add("border-success-subtle")
      listItem.style = "--bs-bg-opacity: .2;"
      listItem.appendChild(div)
    } else {
      const text = document.createElement("text");
      text.textContent = status[0];
      listItem.appendChild(text);
      listItem.appendChild(text)
    }
    players.appendChild(listItem);
  }
  console.log("statuses:", event.payload);
})

await listen('joined', (event) => {
  let joined = event.payload;
  let room_name = joined[0]
  let username = joined[1]

  if (username == document.getElementById("username").value) {
    const div = document.getElementById("room");
    while (div.firstChild) {
      div.removeChild(div.firstChild)
    }
    createPlayersInRoomWidget(div, room_name)
  }
  
  console.log("joined:", joined)
})

await listen('updateGameMode', (event) => {
  let game_mod = event.payload;
  console.log("game_mod:", game_mod);
  document.getElementById("new-game").checked = game_mod == 0;
  document.getElementById("load-game").checked = game_mod == 1;
})

function setupDropdownMenu(buttonId, menuId) {
  const dropdownButton = document.querySelector('#' + buttonId);
  const dropdownMenu = document.querySelector('#' + menuId);
  dropdownMenu.addEventListener('click', function (e) {
    if (e.target.classList.contains('dropdown-item')) {
      const selectedItem = e.target.textContent;
      dropdownButton.textContent = selectedItem;
    }
  });
}
setupDropdownMenu("node-address", "node-addresses");
setupDropdownMenu("room-max-players", "players-count");


function createPlayersInRoomWidget(parentElement, roomName) {
  // Create label element
  const label = document.createElement("label");
  label.className = "form-label";
  label.textContent = "Players in the room";

  // Create div elements
  const divRow1 = document.createElement("div");
  divRow1.className = "row";

  const divCol1 = document.createElement("div");
  divCol1.className = "col";

  const divInner = document.createElement("div");
  divInner.style.height = "150px";
  divInner.className = "bg-dark-subtle rounded-2 overflow-y-auto";

  // Create ul element
  const ul = document.createElement("ul");
  ul.className = "list-group list-group-flush";
  ul.id = "players";

  // Create div elements
  const divRow2 = document.createElement("div");
  divRow2.className = "row pt-3 align-items-start justify-content-start";

  const divCol2 = document.createElement("div");
  divCol2.className = "col";

  // Create form-check elements
  const formCheck1 = document.createElement("div");
  formCheck1.className = "form-check form-check-inline";

  const input1 = document.createElement("input");
  input1.className = "form-check-input";
  input1.type = "radio";
  input1.name = "inlineRadioOptions";
  input1.id = "new-game";
  input1.value = "option1";
  input1.checked = true
  input1.onchange = () =>  hostmode(0)

  const label1 = document.createElement("label");
  label1.className = "form-check-label";
  label1.htmlFor = "inlineRadio1";
  label1.textContent = "New game";

  const formCheck2 = document.createElement("div");
  formCheck2.className = "form-check form-check-inline";

  const input2 = document.createElement("input");
  input2.className = "form-check-input";
  input2.type = "radio";
  input2.name = "inlineRadioOptions";
  input2.id = "load-game";
  input2.value = "option2";
  input2.onchange = () => hostmode(1)

  const label2 = document.createElement("label");
  label2.className = "form-check-label";
  label2.htmlFor = "inlineRadio2";
  label2.textContent = "Load game";

  // Create div elements
  const divRow3 = document.createElement("div");
  divRow3.className = "row pt-3 align-items-start justify-content-start";

  const divCol3 = document.createElement("div");
  divCol3.className = "col col-auto";

  // Create button elements
  const buttonLeave = document.createElement("button");
  buttonLeave.type = "button";
  buttonLeave.className = "btn btn-light";
  buttonLeave.setAttribute("data-bs-toggle", "modal");
  buttonLeave.innerHTML = '<i class="bi bi-plus"></i>Leave';
  buttonLeave.addEventListener("click", () => leave(roomName));

  const buttonReady = document.createElement("button");
  buttonReady.type = "button";
  buttonReady.className = "btn btn-success";
  buttonReady.setAttribute("data-bs-toggle", "modal");
  buttonReady.innerHTML = '<i class="bi bi-plus"></i>Ready';
  buttonReady.addEventListener("click", () => ready(roomName));

  // Append elements to their respective parent elements
  divInner.appendChild(ul);
  divCol1.appendChild(divInner);
  divRow1.appendChild(divCol1);

  formCheck1.appendChild(input1);
  formCheck1.appendChild(label1);

  formCheck2.appendChild(input2);
  formCheck2.appendChild(label2);

  divCol2.appendChild(formCheck1);
  divCol2.appendChild(formCheck2);
  divRow2.appendChild(divCol2);

  divCol3.appendChild(buttonLeave);
  divCol3.appendChild(buttonReady);
  divRow3.appendChild(divCol3);

  parentElement.appendChild(label);
  parentElement.appendChild(divRow1);
  parentElement.appendChild(divRow2);
  parentElement.appendChild(divRow3);
}
