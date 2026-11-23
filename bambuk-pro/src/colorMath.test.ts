import test from "node:test";
import assert from "node:assert/strict";
import { colorMatrix, filterMatrix, fullMatrix, svgValues } from "./colorMath.ts";

const neutralC = { saturation: 0, vibrance: 0, contrast: 0, brightness: 0, gamma: 1, temperature: 0, tint: 0, sharpness: 0 };
const t = () => ({ enabled: false, amount: 100 });
const neutralF = { grayscale: t(), sepia: t(), invert: t(), blueCut: t(), highContrast: t(), redBoost: t() };

test("нейтральные параметры → единичная матрица", () => {
  const m = fullMatrix(neutralC, neutralF);
  for (let i = 0; i < 5; i++) for (let j = 0; j < 5; j++) assert.ok(Math.abs(m[i][j] - (i === j ? 1 : 0)) < 1e-9);
});
test("яркость сдвигает перенос на 0.3 при +100", () => {
  assert.ok(Math.abs(colorMatrix({ ...neutralC, brightness: 100 })[4][0] - 0.3) < 1e-9);
});
test("теплее: больше красного, меньше синего (совпадает с Rust)", () => {
  const m = colorMatrix({ ...neutralC, temperature: 100 });
  assert.ok(Math.abs(m[0][0] - 1.25) < 1e-9 && Math.abs(m[2][2] - 0.65) < 1e-9);
});
test("серый фильтр уравнивает строки яркости", () => {
  const m = filterMatrix({ ...neutralF, grayscale: { enabled: true, amount: 100 } });
  assert.ok(Math.abs(m[0][0] - 0.2126) < 1e-9 && Math.abs(m[0][1] - 0.2126) < 1e-9);
});
test("svg-строка содержит 20 чисел", () => {
  assert.equal(svgValues(fullMatrix(neutralC, neutralF)).split(" ").length, 20);
});
