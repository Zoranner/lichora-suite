using System;
using Unity.Collections;
using UnityEngine;
using UnityEngine.UI;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserSurface : IDisposable
    {
        private readonly RawImage _RawImage;
        private BrowserRenderSettings _Settings;
        private Texture2D _Texture;
        private Material _Material;
        private NativeArray<byte> _TextureBuffer;

        public BrowserSurface(RawImage rawImage, BrowserRenderSettings settings)
        {
            _RawImage = rawImage ?? throw new ArgumentNullException(nameof(rawImage));
            _Settings = settings;
        }

        public int TextureWidth => _Texture != null ? _Texture.width : 0;
        public int TextureHeight => _Texture != null ? _Texture.height : 0;

        public void Initialize(int width, int height)
        {
            EnsureTextureSize(width, height);
            ApplyMaterial();
        }

        public bool EnsureTextureSize(int width, int height)
        {
            if (width <= 0 || height <= 0)
            {
                return false;
            }

            if (_Texture != null && _Texture.width == width && _Texture.height == height)
            {
                return false;
            }

            _TextureBuffer = default;

            if (_Texture != null)
            {
                UnityEngine.Object.Destroy(_Texture);
            }

            _Texture = new Texture2D(width, height, TextureFormat.BGRA32, false, false)
            {
                filterMode = FilterMode.Bilinear,
            };
            _TextureBuffer = _Texture.GetRawTextureData<byte>();
            _RawImage.texture = _Texture;

            if (_Material != null)
            {
                _Material.mainTexture = _Texture;
            }

            return true;
        }

        public bool TryGetTextureBuffer(out NativeArray<byte> buffer)
        {
            buffer = _TextureBuffer;
            return _Texture != null && _TextureBuffer.IsCreated;
        }

        public void Apply()
        {
            if (_Texture == null || !_TextureBuffer.IsCreated)
            {
                return;
            }

            _Texture.Apply(false);
        }

        public void UpdateSettings(BrowserRenderSettings settings)
        {
            _Settings = settings;
            ApplyMaterial();
        }

        public void Dispose()
        {
            _TextureBuffer = default;

            if (_Material != null)
            {
                UnityEngine.Object.Destroy(_Material);
                _Material = null;
            }

            if (_Texture != null)
            {
                UnityEngine.Object.Destroy(_Texture);
                _Texture = null;
            }

            _RawImage.texture = null;
            _RawImage.material = null;
        }

        private void ApplyMaterial()
        {
            var renderMaterial = Resources.Load<Material>(_Settings.MaterialResourcePath);
            if (renderMaterial == null)
            {
                Debug.LogWarning(
                    $"[BrowserSurface] Material '{_Settings.MaterialResourcePath}' not found, fallback to RawImage default material."
                );
                _RawImage.material = null;
                _RawImage.uvRect = new Rect(0f, 1f, 1f, -1f);
                return;
            }

            _RawImage.uvRect = new Rect(0f, 0f, 1f, 1f);

            if (_Material == null || _Material.shader != renderMaterial.shader)
            {
                if (_Material != null)
                {
                    UnityEngine.Object.Destroy(_Material);
                }

                _Material = UnityEngine.Object.Instantiate(renderMaterial);
            }

            _Material.mainTexture = _Texture;
            _Material.SetFloat("_FlipY", _Settings.FlipY ? 1f : 0f);
            _Material.SetFloat(
                "_UseBrowserAlpha",
                _Settings.TransparencyMode == BrowserTransparencyMode.BrowserAlpha ? 1f : 0f
            );
            _Material.SetColor("_FilterColor", _Settings.FilterColor);
            _Material.SetFloat("_ColorThreshold", GetColorThreshold());
            _RawImage.material = _Material;
        }

        private float GetColorThreshold()
        {
            return Mathf.Max(0f, _Settings.ColorThreshold);
        }
    }
}
