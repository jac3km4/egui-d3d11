struct vs_in {
    float3 position : POSITION;
};

struct vs_out {
    float4 clip : SV_POSITION; // required output of VS
};

vs_out vs_main(vs_in input) {
  vs_out output;

  output.clip = float4(input.position, 1.0);

  return output;
}

float4 ps_main(vs_out input) : SV_TARGET {
  return float4(1.0, 0.0, 1.0, 1.0);
}