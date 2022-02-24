struct vs_in {
  float2 position : POSITION;
  float2 uv : TEXCOORD;
  float4 color : COLOR;
  uint mode : MODE;
};

struct vs_out {
  float4 clip : SV_POSITION;
  float4 color : COLOR;
  float2 uv : TEXCOORD;
  uint mode : MODE;
};

vs_out vs_main(vs_in input) {
  vs_out output;

  output.clip = float4(input.position, 0.0, 1.0);
  output.color = input.color;
  output.uv = input.uv;
  output.mode = input.mode;

  return output;
}

sampler sampler0;
Texture2D texture0;

float4 ps_main(vs_out input) : SV_TARGET {
  if (input.mode == 0) {
    float3 albedo = pow(
      input.color.xyz,
      (1.0 / 2.2).xxx
    );
    float alpha = input.color.w * texture0.Sample(sampler0, input.uv).x;

    return float4(albedo, alpha);
  } else {
    float4 color = pow(texture0.Sample(sampler0, input.uv) * input.color, (1.0 / 2.2).xxxx);

    return color;
  }
}