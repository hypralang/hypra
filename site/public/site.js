// A copy button on every code block.
for (const pre of document.querySelectorAll(".codeblock pre")) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "copy";
  button.textContent = "Copy";
  button.addEventListener("click", async () => {
    await navigator.clipboard.writeText(pre.innerText.trimEnd());
    button.textContent = "Copied";
    setTimeout(() => (button.textContent = "Copy"), 1500);
  });
  pre.parentElement.append(button);
}
