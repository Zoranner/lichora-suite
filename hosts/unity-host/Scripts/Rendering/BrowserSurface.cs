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
        private Texture2D _OwnershipMaskTexture;
        private Material _Material;
        private NativeArray<byte> _TextureBuffer;
        private byte[] _OwnershipMaskValues = Array.Empty<byte>();
        private byte[] _OwnershipMaskPixels = Array.Empty<byte>();
        private int _OwnershipMaskBytesPerPixel = 1;
        private ulong _OwnershipMaskRevision = ulong.MaxValue;

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

        internal void UpdateOwnershipMask(InputOwnershipRenderSnapshot snapshot)
        {
            if (!_Settings.UseOwnershipMask || _Texture == null)
            {
                SetOwnershipMaskEnabled(false);
                return;
            }

            if (
                _OwnershipMaskTexture != null
                && _OwnershipMaskTexture.width == _Texture.width
                && _OwnershipMaskTexture.height == _Texture.height
                && _OwnershipMaskRevision == snapshot.Revision
            )
            {
                SetOwnershipMaskEnabled(true);
                return;
            }

            EnsureOwnershipMaskTexture(_Texture.width, _Texture.height);
            if (_OwnershipMaskTexture == null)
            {
                SetOwnershipMaskEnabled(false);
                return;
            }

            OwnershipAlphaMaskBuilder.Build(
                snapshot,
                _OwnershipMaskTexture.width,
                _OwnershipMaskTexture.height,
                _OwnershipMaskValues
            );
            WriteOwnershipMaskPixels();
            _OwnershipMaskTexture.LoadRawTextureData(_OwnershipMaskPixels);
            _OwnershipMaskTexture.Apply(false);
            _OwnershipMaskRevision = snapshot.Revision;

            if (_Material != null)
            {
                _Material.SetTexture("_OwnershipMaskTex", _OwnershipMaskTexture);
            }

            SetOwnershipMaskEnabled(true);
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

            if (_OwnershipMaskTexture != null)
            {
                UnityEngine.Object.Destroy(_OwnershipMaskTexture);
                _OwnershipMaskTexture = null;
            }

            _RawImage.texture = null;
            _RawImage.material = null;
        }

        private void EnsureOwnershipMaskTexture(int width, int height)
        {
            if (width <= 0 || height <= 0)
            {
                return;
            }

            if (
                _OwnershipMaskTexture != null
                && _OwnershipMaskTexture.width == width
                && _OwnershipMaskTexture.height == height
            )
            {
                return;
            }

            if (_OwnershipMaskTexture != null)
            {
                UnityEngine.Object.Destroy(_OwnershipMaskTexture);
            }

            var textureFormat = GetOwnershipMaskTextureFormat();
            _OwnershipMaskBytesPerPixel = textureFormat == TextureFormat.RGBA32 ? 4 : 1;
            _OwnershipMaskTexture = new Texture2D(width, height, textureFormat, false, true)
            {
                filterMode = FilterMode.Point,
                wrapMode = TextureWrapMode.Clamp,
            };
            _OwnershipMaskValues = new byte[width * height];
            _OwnershipMaskPixels = new byte[width * height * _OwnershipMaskBytesPerPixel];
            _OwnershipMaskRevision = ulong.MaxValue;

            if (_Material != null)
            {
                _Material.SetTexture("_OwnershipMaskTex", _OwnershipMaskTexture);
            }
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
            if (_OwnershipMaskTexture != null)
            {
                _Material.SetTexture("_OwnershipMaskTex", _OwnershipMaskTexture);
            }
            SetOwnershipMaskEnabled(_Settings.UseOwnershipMask && _OwnershipMaskTexture != null);
            _RawImage.material = _Material;
        }

        private void SetOwnershipMaskEnabled(bool enabled)
        {
            if (_Material == null)
            {
                return;
            }

            _Material.SetFloat("_UseOwnershipMask", enabled ? 1f : 0f);
        }

        private void WriteOwnershipMaskPixels()
        {
            if (_OwnershipMaskBytesPerPixel == 1)
            {
                Array.Copy(_OwnershipMaskValues, _OwnershipMaskPixels, _OwnershipMaskValues.Length);
                return;
            }

            var targetIndex = 0;
            for (var index = 0; index < _OwnershipMaskValues.Length; index++)
            {
                var mask = _OwnershipMaskValues[index];
                _OwnershipMaskPixels[targetIndex++] = byte.MaxValue;
                _OwnershipMaskPixels[targetIndex++] = byte.MaxValue;
                _OwnershipMaskPixels[targetIndex++] = byte.MaxValue;
                _OwnershipMaskPixels[targetIndex++] = mask;
            }
        }

        private static TextureFormat GetOwnershipMaskTextureFormat()
        {
            return SystemInfo.SupportsTextureFormat(TextureFormat.Alpha8)
                ? TextureFormat.Alpha8
                : TextureFormat.RGBA32;
        }

        private float GetColorThreshold()
        {
            return _Settings.TransparencyMode == BrowserTransparencyMode.ColorKey
                ? Mathf.Max(0f, _Settings.ColorThreshold)
                : 0f;
        }
    }
}
