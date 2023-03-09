const SERVER_ADDRESS = "ws://127.0.0.1:8080/";

const status = document.querySelector('#status')
const textInput = document.querySelector('#text')
let clients = [];

let socket = null
let websocketUrl = "";
websocketUrl = SERVER_ADDRESS;

function connect() {
    disconnect()
    socket = new WebSocket(websocketUrl)

    socket.onopen = () => {
        updateConnectionStatus()
    }

    socket.onmessage = (ev) => {
        let [userId, message] = processChatMessage(ev.data);
        addChatMessage(userId, message)

        //new client joined in the room
        if (ev.data.includes("user_id:")) {
            if (ev.data.includes("joined")) {
                clients.push(userId);
            }
            if (ev.data.includes("left")) {
                const remove_index = clients.indexOf(userId);
                if (remove_index > -1) {
                    clients.splice(remove_index, 1);
                }
            }
            updateClientList(clients);
        }
    }

    socket.onclose = () => {
        socket = null
        updateConnectionStatus()
    }
}

function disconnect() {
    if (socket) {
        socket.close()
        socket = null

        updateConnectionStatus()
    }
}

function updateConnectionStatus() {
    if (socket) {
        status.style.backgroundColor = 'transparent'
        status.style.color = 'green'
        status.textContent = `connected`
    } else {
        status.style.backgroundColor = 'red'
        status.style.color = 'white'
        status.textContent = 'disconnected'
    }
}

function updateClientList(clients) {
    const clientList = document.querySelector(".client-list-items");
    clientList.innerHTML = "";
    for (const client of clients) {
        const listItem = document.createElement("li");
        listItem.className = "client-list-item";
        listItem.innerText = client;
        clientList.appendChild(listItem);
    }
    const title = document.getElementById('client-list-title');
    if (clients.length <= 1) {
        title.textContent = "Member:";
    } else {
        title.textContent = "Members:"
    }
}
// Example usage: updateClientList(["User 1", "User 2", "User 3"]);


// Get a reference to the chat form and input elements
const chatForm = document.querySelector(".chat-form");
const chatInput = document.querySelector(".chat-input");

// Function to add a new chat message to the chat messages area
function addChatMessage(sender, message) {
    const chatMessages = document.querySelector(".chat-messages");
    const messageElement = document.createElement("div");
    messageElement.className = "chat-message";
    messageElement.innerHTML = `<strong>${sender}:</strong> ${message}`;
    if (message == "joined in the room" || message == "left the room") {
        messageElement.style.color = "#787878";
    }
    chatMessages.appendChild(messageElement);
    chatMessages.scrollTop = chatMessages.scrollHeight;
}

function processChatMessage(data) {
    //[user_id:41 08:16:37]: test message

    // Extract user ID
    const userIdRegex = /\[user_id:(\d+)\s/;
    const userIdMatch = data.match(userIdRegex);
    const userId = userIdMatch ? userIdMatch[1] : null;

    // Extract message
    const messageRegex = /]:\s(.+)/;
    const messageMatch = data.match(messageRegex);
    const message = messageMatch ? messageMatch[1] : null;

    return [userId, message];
}


// Example usage: addChatMessage("User 1", "Hello world!");

// Add an event listener to the chat form for when it is submitted
chatForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const message = chatInput.value.trim();
    chatInput.value = "";

    const text = message
    updateClientList("User", message)
    //logStatus('Sending: ' + text)
    socket.send(text)

    textInput.value = ''
    textInput.focus()
});

//connect the server after enter the chat room
socket?disconnect():connect()

updateConnectionStatus()