    it('should add the filename and filename* params', function(done){
      var app = express();

      app.use(function(req, res){
        res.attachment('/locales/日本語.txt');
        res.send('japanese');
      });

      request(app)
      .get('/')
      .expect('Content-Disposition', 'attachment; filename="???.txt"; filename*=UTF-8\'\'%E6%97%A5%E6%9C%AC%E8%AA%9E.txt')
      .expect(200, done);
    })

    it('should encode latin1 file names with an ASCII fallback and filename* param', function(done){
      var app = express();

      app.use(function(req, res){
        res.attachment('/locales/café.txt');
        res.send('coffee');
      });

      request(app)
      .get('/')
      .expect('Content-Disposition', 'attachment; filename="caf?.txt"; filename*=UTF-8\'\'caf%C3%A9.txt')
      .expect(200, done);
    })
