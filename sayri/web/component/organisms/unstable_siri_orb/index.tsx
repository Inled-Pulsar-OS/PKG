import React, { memo, useEffect, useMemo, useState } from "react";
import { View, StyleSheet } from "react-native";
import {
  Canvas,
  Skia,
  Shader,
  Fill,
} from "@shopify/react-native-skia";
import type { IUnstableSiriORB } from "./types";
import { SHADER_SOURCE } from "./conf";

export const UnstableSiriOrb: React.FC<IUnstableSiriORB> = memo<IUnstableSiriORB>(
  ({
    size = 300,
    speed = 1,
    primaryColor = { r: 0.4, g: 0.6, b: 1.0 },
    secondaryColor = { r: 0.0, g: 0.8, b: 0.8 },
    noiseIntensity = 1,
    glowIntensity = 1.5,
    saturation = 2,
    brightness = 1,
    rotationSpeed = 1,
    noiseScale = 3,
    coreIntensity = 0.5,
    edgeSoftness = 0.04,
    paused = false,
    style,
  }) => {
    const [time, setTime] = useState(0);

    useEffect(() => {
      if (paused) return;
      let animId: number;
      let start = performance.now();
      const loop = (now: number) => {
        setTime(((now - start) / 1000.0) * speed);
        animId = requestAnimationFrame(loop);
      };
      animId = requestAnimationFrame(loop);
      return () => cancelAnimationFrame(animId);
    }, [speed, paused]);

    const shader = useMemo(() => {
      try {
        return Skia.RuntimeEffect.Make(SHADER_SOURCE);
      } catch (err) {
        console.error("Shader compilation failed:", err);
        return null;
      }
    }, []);

    const uniforms = useMemo(
      () => ({
        iResolution: [size, size],
        iTime: time,
        primaryColor: [primaryColor.r, primaryColor.g, primaryColor.b],
        secondaryColor: [secondaryColor.r, secondaryColor.g, secondaryColor.b],
        noiseIntensity,
        glowIntensity,
        saturation,
        brightness,
        rotationSpeed,
        noiseScale,
        coreIntensity,
        edgeSoftness,
      }),
      [
        size,
        time,
        primaryColor.r,
        primaryColor.g,
        primaryColor.b,
        secondaryColor.r,
        secondaryColor.g,
        secondaryColor.b,
        noiseIntensity,
        glowIntensity,
        saturation,
        brightness,
        rotationSpeed,
        noiseScale,
        coreIntensity,
        edgeSoftness,
      ]
    );

    if (!shader) return null;

    return (
      <View style={[styles.container, { width: size, height: size }, style]}>
        <Canvas style={styles.canvas}>
          <Fill>
            <Shader source={shader} uniforms={uniforms} />
          </Fill>
        </Canvas>
      </View>
    );
  }
);

const styles = StyleSheet.create({
  container: {
    overflow: "hidden",
    borderRadius: 1000,
  },
  canvas: {
    flex: 1,
  },
});

export default UnstableSiriOrb;
