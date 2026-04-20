using System.Collections.Generic;
using System.Linq;

namespace Examples.Repositories
{
    // Type-4 clone of ProductRepository. Exactly the same behaviour but
    // rewritten via LINQ + expression-bodied members. Structural and
    // token-level signals miss this entirely — only the embedding pass
    // surfaces the semantic equivalence.
    public sealed class ProductRepositoryLinq
    {
        private readonly List<Product> store = new List<Product>();

        public Product? FindById(int sku) => store.FirstOrDefault(item => item.Sku == sku);

        public IReadOnlyList<Product> FindAll() => store.ToList();

        public void Insert(Product entity)
        {
            if (entity is null)
            {
                throw new System.ArgumentNullException(nameof(entity));
            }

            store.Add(entity);
        }

        public bool Delete(int sku)
        {
            var match = store.FirstOrDefault(item => item.Sku == sku);
            if (match is null)
            {
                return false;
            }

            _ = store.Remove(match);
            return true;
        }

        public int Count() => store.Count;
    }
}
