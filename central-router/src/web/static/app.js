const e = {
  main: document.querySelector("main"),
  nodes: {
    table: document.querySelector("main table"),
    body: document.querySelector("main table tbody"),
    footer: {
      container: document.querySelector("main table tfoot"),
      button: document.querySelector("main table tfoot button"),
    },
  },
  status: {
    container: document.querySelector(".status"),
    button: document.querySelector(".status button"),
  },
  refreshed: {
    container: document.querySelector(".refreshed"),
    time: document.querySelector(".refreshed span"),
  },
};

const s = {
  loading: false,
  status: false,
  nodes: [],
};

const run = () => {
  const refresh = () => {
    if (s.loading) {
      return;
    }

    s.loading = true;
    renderLoading();
    refreshNodes();
    refreshStatus();
    updateRefreshedAt();
    s.loading = false;
    renderLoading();
  };
  setInterval(refresh, 1000);
  refresh();

  e.status.button.addEventListener("click", toggleStatus);
  e.nodes.footer.button.addEventListener("click", toggleRegistered);
};

const isRegistered = () => {
  return s.nodes.some((i) => i.you);
};

const refreshNodes = () => {
  fetch("/api/nodes")
    .then((r) => r.json())
    .then((r) => (s.nodes = r))
    .then(renderNodes);
};

const join = (name) => {
  fetch("/api/nodes", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name: name }),
  }).then(refreshNodes);
};

const leave = () => {
  fetch("/api/nodes", {
    method: "DELETE",
  }).then(refreshNodes);
};

const reorder = (curIndex, newIndex) => {
  fetch("/api/nodes", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ cur_i: curIndex, new_i: newIndex }),
  }).then(refreshNodes);
};

const refreshStatus = () => {
  fetch("/api/status")
    .then((r) => r.json())
    .then((r) => (s.status = r.on))
    .then(renderStatus);
};

const toggleStatus = () => {
  fetch("/api/status", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ on: !s.status }),
  }).then(refreshStatus);
};

const toggleRegistered = () => {
  if (isRegistered()) {
    leave();
  } else {
    const name = prompt("Please enter your name", localStorage.getItem("defaultName") || "");
    if (name) {
        localStorage.setItem("defaultName", name)
      join(name);
    }
  }
};

const updateRefreshedAt = () => {
  e.refreshed.time.innerText = new Date().toISOString();
};

const renderLoading = () => {
  e.main.classList.toggle("loading", s.loading);
};

const renderNodes = () => {
  let html = s.nodes
    .map(
      (n, i) => `<tr class="${n.you ? "you" : ""}">
            <td>${i + 1}</td>
            <td>${escapeHtml(n.name.substring(0, 20))}</td>
            <td>${n.mac || 'N/A'}</td>
            <td>${n.ip}</td>
            <td>${new Date(n.created.secs_since_epoch * 1000).toISOString()}</td>
            <td>
                <button class="up">&uarr;</button>
                <button class="down">&darr;</button>
            </td>
        </tr>`
    )
    .join(`\n`);

  if (!html) {
    html = `<tr class="none"><td colspan="100">No nodes have connected</td></tr>`;
  }

  e.nodes.body.innerHTML = html;
  e.nodes.footer.container.classList.toggle("registered", isRegistered());
  e.nodes.footer.button.innerHTML = isRegistered() ? "Unregister" : "Register";

  registerNodeHandlers()
};


const renderStatus = () => {
  e.status.container.classList.toggle("on", s.status);
  e.status.container.classList.toggle("off", !s.status);
  e.status.button.innerText = `Status: ${s.status ? "ON" : "OFF"}`;
};

const registerNodeHandlers = () => {
    for (let i = 0; i < s.nodes.length; i++) {
        const row = e.nodes.body.children.item(i);
        const up = row.querySelector(".up")
        const down = row.querySelector(".down")

        const idx = i;
        up.addEventListener("click", () => reorder(idx, idx - 1));
        down.addEventListener("click", () => reorder(idx, idx + 1));
        up.disabled = idx === 0;
        down.disabled = idx === s.nodes.length - 1;
    }
}

const escapeHtml = (str) => {
  let div = document.createElement("div");
  div.innerText = str;
  return div.innerHTML;
};

run();
