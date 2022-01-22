struct vs_in {
    float2 position : POSITION;
    float2 uv : TEXCOORD;
    uint4 color : COLOR;
};

struct vs_out {
    float4 clip : SV_POSITION;
    uint4 color : COLOR;
};

vs_out vs_main(vs_in input) {
  vs_out output;

  output.clip = float4(input.position, 0.0, 1.0);
  output.color = input.color;

  return output;
}

uint4 ps_main(vs_out input) : SV_TARGET {
  return input.color;
}