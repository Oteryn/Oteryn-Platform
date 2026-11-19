const descriptions = {
  rycerz: "Walczy w zwarciu. Ciężka zbroja pozwala mu przyjąć ciosy w pierwszej linii.",
  paladyn: "Wybiera walkę na dystans. Z łukiem w dłoni trzyma przeciwnika na odległość.",
  druid: "Czerpie moc z natury. Łączy magię z leczeniem i wsparciem towarzyszy.",
  czarodziej: "Polega na zaklęciach ofensywnych. Atakuje z dystansu, gdy drużyna wiąże wroga w walce.",
  mnich: "Walczy w zwarciu. Zamiast ciężkiego oręża wykorzystuje siłę i precyzję uderzeń.",
};

const portraits = document.querySelector(".portraits");
const stylePicker = document.querySelector("#styl");
const description = document.querySelector("#opis");

function applyStyle() {
  for (const card of portraits.querySelectorAll("label")) {
    const profession = card.querySelector("input").value;
    card.querySelector("img").src = `${stylePicker.value}/${profession}.webp`;
  }
}

function showDescription() {
  const selected = portraits.querySelector(":checked");
  description.textContent = descriptions[selected.value];
}

stylePicker.addEventListener("change", applyStyle);
portraits.addEventListener("change", showDescription);
window.addEventListener("pageshow", () => {
  applyStyle();
  showDescription();
});
