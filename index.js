// fetch("/api/search", {
//   method: "POST",
//   headers: {
//     "Content-Type": "text/plain",
//   },
//   mode: "cors",
//   body: "lol what a stupid guy",
// }).then((res) => console.log(res));
async function search(prompt) {
  const results = document.getElementById("results");

  results.innerHTML = "";

  const response = await fetch("/api/search", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: prompt,
  });
  const json = await response.json();

  results.innerHTML = "";
}

let query = document.getElementById("results");
let currentSearch = Promise.resolve();

if (query) {
  query.addEventListener("keypress", (e) => {
    if (e.key == "Enter") {
      currentSearch.then(() => search(query));
    }
  });
}
