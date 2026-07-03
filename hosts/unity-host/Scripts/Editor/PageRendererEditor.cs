// ============================================================
// Project: EmbeddedBrowser
// Author: yangxinran@EN01-210826-09
// Datetime: 2024-04-08 10:15:32
// Description: Inspector binding for PageRenderer.
// ============================================================

using UnityEditor;

namespace KimoTech.LichoraHost.Editors
{
    [CanEditMultipleObjects]
    [CustomEditor(typeof(PageRenderer))]
    public class PageRendererEditor : Editor { }
}
