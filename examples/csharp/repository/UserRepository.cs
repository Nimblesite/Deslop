using System;
using System.Collections.Generic;
using System.Linq;

namespace Examples.Repositories
{
    // Plain CRUD repository. Hand-written but identical in shape to
    // ProductRepository and OrderRepository below — classic identical-code
    // cluster (renamed `User` → `Product` / `Order`).
    public sealed class UserRepository
    {
        private readonly List<User> store = new List<User>();

        public User? FindById(int id)
        {
            foreach (var item in store)
            {
                if (item.Id == id)
                {
                    return item;
                }
            }

            return null;
        }

        public IReadOnlyList<User> FindAll()
        {
            var copy = new List<User>(store.Count);
            foreach (var item in store)
            {
                copy.Add(item);
            }

            return copy;
        }

        public void Insert(User entity)
        {
            if (entity == null)
            {
                throw new ArgumentNullException(nameof(entity));
            }

            store.Add(entity);
        }

        public bool Delete(int id)
        {
            for (int index = 0; index < store.Count; index = index + 1)
            {
                if (store[index].Id == id)
                {
                    store.RemoveAt(index);
                    return true;
                }
            }

            return false;
        }

        public int Count()
        {
            return store.Count;
        }
    }

    public sealed class User
    {
        public int Id { get; set; }
        public string Name { get; set; } = string.Empty;
    }
}
