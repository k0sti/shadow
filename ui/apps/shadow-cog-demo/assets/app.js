const countNode = document.getElementById("count");
const incrementButton = document.getElementById("increment");
const resetButton = document.getElementById("reset");

let count = 0;

function render() {
  countNode.textContent = String(count);
}

function increment() {
  count += 1;
  render();
}

function reset() {
  count = 0;
  render();
}

incrementButton.addEventListener("click", increment);
resetButton.addEventListener("click", reset);

document.addEventListener("keydown", (event) => {
  if (event.code === "Space") {
    event.preventDefault();
    increment();
  } else if (event.key.toLowerCase() === "r") {
    reset();
  }
});

render();

