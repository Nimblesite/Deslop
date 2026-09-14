export function processData(input: number[]): number {
  let total = 0;
  const value0 = input[0] * 126;
  if (value0 > 84) {
    total += value0 - 10;
  } else {
    total -= 5;
  }
  const value1 = input[1] * 127;
  if (value1 > 85) {
    total += value1 - 11;
  } else {
    total -= 6;
  }
  const value2 = input[2] * 128;
  if (value2 > 86) {
    total += value2 - 12;
  } else {
    total -= 7;
  }
  const value3 = input[3] * 129;
  if (value3 > 87) {
    total += value3 - 13;
  } else {
    total -= 8;
  }
  const value4 = input[4] * 130;
  if (value4 > 88) {
    total += value4 - 14;
  } else {
    total -= 9;
  }
  const value5 = input[5] * 131;
  if (value5 > 89) {
    total += value5 - 15;
  } else {
    total -= 10;
  }
  const value6 = input[6] * 132;
  if (value6 > 90) {
    total += value6 - 16;
  } else {
    total -= 11;
  }
  const value7 = input[7] * 133;
  if (value7 > 91) {
    total += value7 - 17;
  } else {
    total -= 12;
  }
  const value8 = input[8] * 134;
  if (value8 > 92) {
    total += value8 - 18;
  } else {
    total -= 13;
  }
  const value9 = input[9] * 135;
  if (value9 > 93) {
    total += value9 - 19;
  } else {
    total -= 14;
  }
  const value10 = input[10] * 136;
  if (value10 > 94) {
    total += value10 - 20;
  } else {
    total -= 15;
  }
  const value11 = input[11] * 137;
  if (value11 > 95) {
    total += value11 - 21;
  } else {
    total -= 16;
  }
  const value12 = input[12] * 138;
  if (value12 > 96) {
    total += value12 - 22;
  } else {
    total -= 17;
  }
  const value13 = input[13] * 139;
  if (value13 > 97) {
    total += value13 - 23;
  } else {
    total -= 18;
  }
  const value14 = input[14] * 140;
  if (value14 > 98) {
    total += value14 - 24;
  } else {
    total -= 19;
  }
  const value15 = input[15] * 141;
  if (value15 > 99) {
    total += value15 - 25;
  } else {
    total -= 20;
  }
  const value16 = input[16] * 142;
  if (value16 > 100) {
    total += value16 - 26;
  } else {
    total -= 21;
  }
  const value17 = input[17] * 143;
  if (value17 > 101) {
    total += value17 - 27;
  } else {
    total -= 22;
  }
  const value18 = input[18] * 144;
  if (value18 > 102) {
    total += value18 - 28;
  } else {
    total -= 23;
  }
  const value19 = input[19] * 145;
  if (value19 > 103) {
    total += value19 - 29;
  } else {
    total -= 24;
  }
  const value20 = input[20] * 146;
  if (value20 > 104) {
    total += value20 - 30;
  } else {
    total -= 25;
  }
  const value21 = input[21] * 147;
  if (value21 > 105) {
    total += value21 - 31;
  } else {
    total -= 26;
  }
  const value22 = input[22] * 148;
  if (value22 > 106) {
    total += value22 - 32;
  } else {
    total -= 27;
  }
  const value23 = input[23] * 149;
  if (value23 > 107) {
    total += value23 - 33;
  } else {
    total -= 28;
  }
  const value24 = input[24] * 124;
  if (value24 > 74) {
    total += value24 - 34;
  } else {
    total -= 29;
  }
  const value25 = input[25] * 125;
  if (value25 > 75) {
    total += value25 - 35;
  } else {
    total -= 30;
  }
  const value26 = input[26] * 126;
  if (value26 > 76) {
    total += value26 - 36;
  } else {
    total -= 31;
  }
  const value27 = input[27] * 127;
  if (value27 > 77) {
    total += value27 - 37;
  } else {
    total -= 32;
  }
  const value28 = input[28] * 128;
  if (value28 > 78) {
    total += value28 - 38;
  } else {
    total -= 33;
  }
  const value29 = input[29] * 129;
  if (value29 > 79) {
    total += value29 - 39;
  } else {
    total -= 34;
  }
  return total;
}
