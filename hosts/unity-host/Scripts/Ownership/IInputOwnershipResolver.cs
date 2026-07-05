using UnityEngine;

namespace KimoTech.LichoraHost
{
    public interface IInputOwnershipResolver
    {
        InputOwner ResolveOwner(Vector2 browserNormalizedPosition);
    }
}
