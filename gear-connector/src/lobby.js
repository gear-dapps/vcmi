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
let roomMaxPlayers;

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
  console.log("NewRoom", roomNameEl.value, roomPasswordEl.value, roomMaxPlayers.value)
  await invoke("new_room", {
    roomName: roomNameEl.value,
    password: roomPasswordEl.value,
    maxPlayers: 2, // TODO!
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
  roomMaxPlayers = document.querySelector("#room-max-players")
  document.querySelector("#new-room-button").addEventListener("click", () => newRoom());
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

await listen('showRooms', (event) => {
  let roomView = document.getElementById("collapseRoom");
  let bsCollapse = new bootstrap.Collapse(roomView);
  console.log("show Rooms:", users);
  bsCollapse.toggle();
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
    listItem.classList.add("bg-primary");
    listItem.classList.add("border-primary-subtle")

    listItem.style = "--bs-bg-opacity: .2;"
    listItem.textContent = users[i];
    list.appendChild(listItem);
  }
})

await listen('addSessions', (event) => {
  let sessions = event.payload;
  console.log("add Session:", sessions);
  const div = document.getElementById("sessions");
  while (div.firstChild) {
    div.removeChild(div.firstChild)
  }
  for (let i = 0; i < sessions.length; ++i) {
    const button = document.createElement("button");
    button.className = "btn";
    button.classList.add("border-primary-subtle")
    
    button.style = "background-color: rgb(0, 0, 50);"
    button.textContent = sessions[i].name;
    button.addEventListener("click", () => joinRoom(button.innerText));

    div.appendChild(button);
    const badge = document.createElement("span");
    badge.classList.add("badge");
    badge.classList.add("text-bg-success");
    badge.textContent = sessions[i].joined + "/" + sessions[i].total
    button.appendChild(badge);
  }
})

await listen('chatMessage', (event) => {
  let messages = event.payload;
  console.log("chat Message:", messages);
  const list = document.getElementById("messages");
  // while (list.firstChild) {
  //   list.removeChild(list.firstChild);
  // }
  const listItem = document.createElement("li");

  listItem.className = "list-group-item";
  const strong = document.createElement("strong");
  strong.textContent = messages[0] + ": ";
  listItem.appendChild(strong);
  const text = document.createElement("text");
  text.textContent = messages[1];
  listItem.appendChild(text);

  listItem.classList.add("bg-primary");
  listItem.classList.add("border-primary-subtle")

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
  let usersCount = event.payload[0];

  for (let i = 0; i < usersCount; ++i) {

  }
  console.log("status:", usersCount);
})

await listen('joined', (event) => {
  let joined = event.payload;
  console.log("joined:", joined);

})




const dropdownItems = document.querySelector('.dropdown-item');
const dropdownButton = document.querySelector('#node-address');

let activeItem = null;

dropdownItems.forEach(item => {
  item.addEventListener('click', () => {
    if (activeItem) {
      activeItem.classList.remove('active');
    }
    item.classList.add('active');
    dropdownButton.textContent = item.textContent;
    activeItem = item;
  });
});
