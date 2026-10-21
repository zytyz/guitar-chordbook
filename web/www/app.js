// The page around the Rust core. Chord rules, search, diagrams and the PDF all
// come from WebAssembly (pkg/, built by scripts/build-web.sh); this file only
// draws the UI and keeps the catalog in localStorage.

import init, {
  Chordbook, newChord, toggle, setFinger, svg, chordShape, fileStem, stringNames, maxFret,
} from "./pkg/chordbook_web.js";

const STORAGE_KEY = "chordbook/catalog";
const FONTS = ["fonts/LiberationSans-Regular.ttf", "fonts/LiberationSans-Bold.ttf"];

const $ = (id) => document.getElementById(id);
const SVG_NS = "http://www.w3.org/2000/svg";

await init();
const STRING_NAMES = stringNames();
const MAX_FRET = maxFret();

let book = loadBook();
$("loading").remove();

// Storage

function loadBook() {
  let saved = null;
  try {
    saved = localStorage.getItem(STORAGE_KEY);
  } catch {
    // Storage blocked (private browsing in some browsers): run without it.
  }
  try {
    return new Chordbook(saved);
  } catch (err) {
    // Keep the unreadable text so nothing is lost, and start empty.
    try { localStorage.setItem(`${STORAGE_KEY}/unreadable`, saved); } catch {}
    queueMicrotask(() => toast(`Your saved chords could not be read (${err.message}).`, { error: true }));
    return new Chordbook();
  }
}

let askedToPersist = false;

function save() {
  try {
    localStorage.setItem(STORAGE_KEY, book.toJson());
  } catch (err) {
    toast(`Could not save in this browser: ${err.message}`, { error: true });
    return;
  }
  // Ask the browser not to clear the catalog when space runs low.
  if (!askedToPersist && navigator.storage?.persist) {
    askedToPersist = true;
    navigator.storage.persist().catch(() => {});
  }
}

// Chord list

function chordName(chord) {
  return chord.name ?? chordShape(chord);
}

function renderList() {
  const chords = book.chords();
  const query = $("search").value;
  const shown = query.trim() ? Array.from(book.search(query)) : chords.map((_, i) => i);

  $("count").textContent = chords.length ? `${chords.length} chord${chords.length === 1 ? "" : "s"}` : "";
  $("empty").hidden = chords.length > 0;
  $("no-match").hidden = chords.length === 0 || shown.length > 0;
  $("no-match").textContent = `No chords match “${query.trim()}”.`;

  const grid = $("grid");
  grid.replaceChildren(...shown.map((index) => {
    const chord = chords[index];
    const item = document.createElement("li");
    const card = document.createElement("button");
    card.type = "button";
    card.className = "card";
    card.setAttribute("aria-label", chordName(chord));
    // The core escapes names and comments, so its SVG is safe to insert.
    card.innerHTML = svg(chord);
    card.addEventListener("click", () => openViewer(index));
    item.append(card);
    return item;
  }));
}

$("search").addEventListener("input", renderList);

// Viewer

function openViewer(index) {
  // Browsing a chord moves it to the front, as in the terminal app.
  book.touch(index);
  save();
  renderList();
  showViewer();
  $("viewer").showModal();
}

function showViewer() {
  const chord = book.chords()[0];
  $("viewer-title").textContent = chordName(chord);
  $("viewer-diagram").innerHTML = svg(chord);
  $("viewer-shape").textContent = chordShape(chord);
}

$("edit").addEventListener("click", () => {
  $("viewer").close();
  openEditor(book.chords()[0], 0);
});

$("delete").addEventListener("click", async () => {
  const chord = book.chords()[0];
  if (!(await confirmAction(`Delete ${chordName(chord)}?`, "Delete"))) return;
  const removed = book.remove(0);
  save();
  $("viewer").close();
  renderList();
  toast(`Deleted ${chordName(removed)}`, {
    action: "Undo",
    onAction: () => {
      book.add(removed);
      save();
      renderList();
    },
  });
});

$("download-svg").addEventListener("click", () => {
  const chord = book.chords()[0];
  download(new Blob([svg(chord)], { type: "image/svg+xml" }), `${fileStem(chord)}.svg`);
});

// Editor

let draft = null;
let editIndex = null;
let triedToSave = false;

function openEditor(chord, index) {
  draft = structuredClone(chord);
  editIndex = index;
  triedToSave = false;
  $("editor-title").textContent = index === null ? "New chord" : `Edit ${chordName(chord)}`;
  $("name").value = chord.name ?? "";
  $("comment").value = chord.comment ?? "";
  document.querySelector('input[name="finger"][value="0"]').checked = true;
  renderBoard();
  check();
  $("editor").showModal();
  if (index === null) $("name").focus();
}

for (const button of [$("new"), $("empty-new")]) {
  button.addEventListener("click", () => openEditor(newChord(), null));
}

/** The draft with the name and comment fields applied. */
function currentDraft() {
  return { ...draft, name: $("name").value.trim() || null, comment: $("comment").value.trim() };
}

function selectedFinger() {
  return Number(document.querySelector('input[name="finger"]:checked').value);
}

function tap(string, fret) {
  const finger = selectedFinger();
  try {
    const dot = draft.strings[string].Fretted;
    if (fret === 0 || finger === 0) {
      draft = toggle(draft, string, fret);
    } else if (dot && dot.fret === fret) {
      // Tapping a dot with its own finger label clears the label.
      draft = setFinger(draft, string, dot.finger === finger ? undefined : finger);
    } else {
      draft = setFinger(toggle(draft, string, fret), string, finger);
    }
  } catch (err) {
    showError(err.message);
    return;
  }
  renderBoard();
  check();
}

/** Shows why the draft could not be saved, live, while the user edits. */
function check() {
  $("editor-shape").textContent = chordShape(draft);
  const chord = currentDraft();
  try {
    book.validate(chord, editIndex ?? undefined);
    showError("");
  } catch (err) {
    // Do not nag about a missing name before the first save attempt.
    showError(chord.name || triedToSave ? err.message : "");
  }
}

function showError(message) {
  $("editor-error").textContent = message ? message[0].toUpperCase() + message.slice(1) : "";
}

$("name").addEventListener("input", check);
$("comment").addEventListener("input", check);

$("editor-form").addEventListener("submit", (event) => {
  event.preventDefault();
  triedToSave = true;
  const chord = currentDraft();
  try {
    if (editIndex === null) book.add(chord);
    else book.replace(editIndex, chord);
  } catch (err) {
    showError(err.message);
    return;
  }
  save();
  $("editor").close();
  $("search").value = "";
  renderList();
  toast(`Saved ${chordName(chord)}`);
});

// Fretboard: strings left to right from low E, like the diagrams, with the
// open and muted markers above the nut.

const CELL_W = 50;
const LEFT = 26;
const NUT_Y = 44;
const FRET_H = 54;
const BOARD_W = LEFT + CELL_W * STRING_NAMES.length + 8;
const BOARD_H = NUT_Y + FRET_H * MAX_FRET + 30;
const stringX = (s) => LEFT + CELL_W * (s + 0.5);
const fretY = (f) => NUT_Y + FRET_H * (f - 0.5);

function el(name, attrs = {}, parent) {
  const node = document.createElementNS(SVG_NS, name);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  parent?.append(node);
  return node;
}

const board = $("board");
let marks;

function buildBoard() {
  board.setAttribute("viewBox", `0 0 ${BOARD_W} ${BOARD_H}`);
  const first = stringX(0);
  const last = stringX(STRING_NAMES.length - 1);
  for (const fret of [3, 5]) {
    el("circle", { class: "inlay", cx: (first + last) / 2, cy: fretY(fret), r: 6 }, board);
  }
  for (let f = 1; f <= MAX_FRET; f++) {
    el("line", { class: "fret", x1: first, x2: last, y1: NUT_Y + FRET_H * f, y2: NUT_Y + FRET_H * f }, board);
    el("text", { class: "label", x: LEFT / 2, y: fretY(f) }, board).textContent = f;
  }
  STRING_NAMES.forEach((name, s) => {
    // Low strings are drawn a little heavier, as on a real neck.
    el("line", {
      class: "string", x1: stringX(s), x2: stringX(s), y1: NUT_Y, y2: NUT_Y + FRET_H * MAX_FRET,
      "stroke-width": 2.6 - s * 0.3,
    }, board);
    el("text", { class: "label", x: stringX(s), y: BOARD_H - 14 }, board).textContent = name;
  });
  el("rect", { class: "nut", x: first - 2, y: NUT_Y - 4, width: last - first + 4, height: 7, rx: 2 }, board);

  marks = el("g", { class: "marks" }, board);

  STRING_NAMES.forEach((name, s) => {
    for (let f = 0; f <= MAX_FRET; f++) {
      const cell = el("g", { class: "cell", role: "button", tabindex: 0 }, board);
      cell.dataset.string = s;
      cell.dataset.fret = f;
      const y = f === 0 ? 0 : NUT_Y + FRET_H * (f - 1);
      el("rect", { x: LEFT + CELL_W * s + 1, y: y + 1, width: CELL_W - 2, height: (f === 0 ? NUT_Y - 6 : FRET_H) - 2 }, cell);
      if (f > 0) el("circle", { class: "ghost", cx: stringX(s), cy: fretY(f), r: 17 }, cell);
      cell.addEventListener("click", () => tap(s, f));
      cell.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          tap(s, f);
        }
      });
    }
  });
}

function renderBoard() {
  marks.replaceChildren();
  draft.strings.forEach((state, s) => {
    const x = stringX(s);
    const markerY = NUT_Y / 2 - 3;
    if (state === "Open") {
      el("circle", { class: "open", cx: x, cy: markerY, r: 8 }, marks);
    } else if (state === "Muted") {
      const d = 7;
      el("path", { class: "muted", d: `M${x - d} ${markerY - d}L${x + d} ${markerY + d}M${x + d} ${markerY - d}L${x - d} ${markerY + d}` }, marks);
    } else {
      const { fret, finger } = state.Fretted;
      el("circle", { class: "dot", cx: x, cy: fretY(fret), r: 17 }, marks);
      if (finger != null) el("text", { class: "finger", x, y: fretY(fret) + 1 }, marks).textContent = finger;
    }
  });
  for (const cell of board.querySelectorAll(".cell")) {
    const s = Number(cell.dataset.string);
    const f = Number(cell.dataset.fret);
    const state = draft.strings[s];
    const name = STRING_NAMES[s];
    let label;
    if (f === 0) label = `${name} string, ${state === "Muted" ? "muted" : state === "Open" ? "open" : "fretted"}. Tap to switch open and muted.`;
    else if (state.Fretted?.fret === f) label = `${name} string, fret ${f}, dot${state.Fretted.finger ? ` with finger ${state.Fretted.finger}` : ""}`;
    else label = `${name} string, fret ${f}`;
    cell.setAttribute("aria-label", label);
  }
}

buildBoard();

// Closing sheets

for (const button of document.querySelectorAll("[data-close]")) {
  button.addEventListener("click", () => button.closest("dialog").close());
}
for (const dialog of document.querySelectorAll("dialog.sheet")) {
  // A tap on the backdrop closes the sheet.
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });
}

function confirmAction(message, okLabel) {
  const dialog = $("confirm");
  $("confirm-text").textContent = message;
  $("confirm-ok").textContent = okLabel;
  dialog.returnValue = "";
  dialog.showModal();
  return new Promise((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue === "ok"), { once: true });
  });
}

// Menu: export, backup, restore

const menu = $("menu");
document.addEventListener("click", (event) => {
  if (menu.open && !menu.contains(event.target)) menu.open = false;
});
menu.querySelector(".menu-list").addEventListener("click", () => { menu.open = false; });

function download(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

let fontBytes = null;
const booksWithFonts = new WeakSet();

$("export-pdf").addEventListener("click", async () => {
  if (book.chords().length === 0) {
    toast("Add a chord first");
    return;
  }
  try {
    fontBytes ??= await Promise.all(FONTS.map(async (path) => {
      const response = await fetch(path);
      if (!response.ok) throw new Error(`could not load ${path}`);
      return new Uint8Array(await response.arrayBuffer());
    }));
    if (!booksWithFonts.has(book)) {
      for (const font of fontBytes) book.addFont(font);
      booksWithFonts.add(book);
    }
    download(new Blob([book.catalogPdf()], { type: "application/pdf" }), "chords.pdf");
  } catch (err) {
    fontBytes = null;
    toast(`Could not make the PDF: ${err.message}`, { error: true });
  }
});

$("backup").addEventListener("click", () => {
  const date = new Date().toISOString().slice(0, 10);
  download(new Blob([book.toJson()], { type: "application/json" }), `chordbook-${date}.json`);
});

$("restore").addEventListener("click", () => $("restore-file").click());

$("restore-file").addEventListener("change", async (event) => {
  const file = event.target.files[0];
  event.target.value = "";
  if (!file) return;
  let restored;
  try {
    restored = new Chordbook(await file.text());
  } catch (err) {
    toast(`${file.name} is not a chord catalog`, { error: true });
    return;
  }
  const have = book.chords().length;
  const count = restored.chords().length;
  const plural = (n) => `${n} chord${n === 1 ? "" : "s"}`;
  if (have > 0 && !(await confirmAction(`Replace your ${plural(have)} with the ${plural(count)} in ${file.name}?`, "Replace"))) {
    restored.free();
    return;
  }
  book.free();
  book = restored;
  save();
  $("search").value = "";
  renderList();
  toast(`Restored ${plural(count)}`);
});

// Toast

let toastTimer;
let toastHandler;

function toast(message, { error = false, action, onAction } = {}) {
  const box = $("toast");
  $("toast-text").textContent = message;
  box.classList.toggle("error", error);
  const button = $("toast-action");
  button.hidden = !action;
  button.textContent = action ?? "";
  toastHandler = onAction;
  box.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { box.hidden = true; }, action || error ? 6000 : 2500);
}

$("toast-action").addEventListener("click", () => {
  $("toast").hidden = true;
  toastHandler?.();
});

renderList();

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("sw.js").catch(() => {});
}
